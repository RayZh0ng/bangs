// Now-playing bridge for macOS.
//
// Since macOS 15.4 MediaRemote only answers processes that carry Apple's
// entitlement, so this library is loaded into /usr/bin/perl (see
// src/media/macos.rs) and talks to the Rust host over stdio:
//   stdout: one JSON object per line describing the current session
//   stdin:  one command per line (toggle | next | previous | seek <seconds>)
// The process exits when stdin closes, so it never outlives the app.

#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>
#include <dlfcn.h>
#include <stdio.h>

typedef void (^NotchInfoBlock)(CFDictionaryRef);
typedef void (^NotchPlayingBlock)(Boolean);
typedef void (^NotchPIDBlock)(int);

static void (*MRGetNowPlayingInfo)(dispatch_queue_t, NotchInfoBlock);
static void (*MRGetIsPlaying)(dispatch_queue_t, NotchPlayingBlock);
static void (*MRGetPID)(dispatch_queue_t, NotchPIDBlock);
static void (*MRRegisterForNotifications)(dispatch_queue_t);
static Boolean (*MRSendCommand)(int, CFDictionaryRef);
static void (*MRSetElapsedTime)(double);

static NSData *lastArtwork;
static NSString *lastLine;
static BOOL refreshQueued;

enum { kCommandTogglePlayPause = 2, kCommandNextTrack = 4, kCommandPreviousTrack = 5 };

static void writeLine(NSDictionary *payload) {
    NSData *json = [NSJSONSerialization dataWithJSONObject:payload options:0 error:nil];
    if (!json) return;
    NSString *line = [[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding];
    if ([line isEqualToString:lastLine]) return;
    lastLine = line;
    fwrite(json.bytes, 1, json.length, stdout);
    fputc('\n', stdout);
    fflush(stdout);
}

static id numberOrNull(id value) {
    return [value isKindOfClass:[NSNumber class]] ? value : [NSNull null];
}

static NSString *stringOrEmpty(id value) {
    return [value isKindOfClass:[NSString class]] ? value : @"";
}

static NSString *mimeForImage(NSData *data) {
    const unsigned char *bytes = data.bytes;
    if (data.length > 4 && bytes[0] == 0x89 && bytes[1] == 'P') return @"image/png";
    return @"image/jpeg";
}

static void publish(NSDictionary *info, Boolean playing, int pid) {
    NSString *title = stringOrEmpty(info[@"kMRMediaRemoteNowPlayingInfoTitle"]);
    if (info.count == 0 || title.length == 0) {
        lastArtwork = nil;
        writeLine(@{ @"available": @NO });
        return;
    }

    NSRunningApplication *app = pid > 0
        ? [NSRunningApplication runningApplicationWithProcessIdentifier:pid]
        : nil;
    NSDate *timestamp = info[@"kMRMediaRemoteNowPlayingInfoTimestamp"];
    double timestampMs = [timestamp isKindOfClass:[NSDate class]]
        ? timestamp.timeIntervalSince1970 * 1000.0
        : NSDate.date.timeIntervalSince1970 * 1000.0;

    NSMutableDictionary *payload = [@{
        @"available": @YES,
        @"title": title,
        @"artist": stringOrEmpty(info[@"kMRMediaRemoteNowPlayingInfoArtist"]),
        @"album": stringOrEmpty(info[@"kMRMediaRemoteNowPlayingInfoAlbum"]),
        @"duration": numberOrNull(info[@"kMRMediaRemoteNowPlayingInfoDuration"]),
        @"elapsed": numberOrNull(info[@"kMRMediaRemoteNowPlayingInfoElapsedTime"]),
        @"timestamp": @(timestampMs),
        // A boxed ternary becomes an integer; the literals stay JSON booleans.
        @"playing": playing ? @YES : @NO,
        @"sourceId": app.bundleIdentifier ?: @"",
        @"appName": app.localizedName ?: @"",
    } mutableCopy];

    NSData *artwork = info[@"kMRMediaRemoteNowPlayingInfoArtworkData"];
    if ([artwork isKindOfClass:[NSData class]] && artwork.length > 0) {
        payload[@"hasArtwork"] = @YES;
        if (![artwork isEqualToData:lastArtwork]) {
            lastArtwork = artwork;
            // Force the line through even if every other field is unchanged.
            lastLine = nil;
            payload[@"artwork"] = [NSString stringWithFormat:@"data:%@;base64,%@",
                mimeForImage(artwork), [artwork base64EncodedStringWithOptions:0]];
        }
    } else {
        lastArtwork = nil;
        payload[@"hasArtwork"] = @NO;
    }
    writeLine(payload);
}

static void refresh(void) {
    dispatch_queue_t queue = dispatch_get_main_queue();
    MRGetNowPlayingInfo(queue, ^(CFDictionaryRef rawInfo) {
        NSDictionary *info = rawInfo ? [(__bridge NSDictionary *)rawInfo copy] : @{};
        MRGetIsPlaying(queue, ^(Boolean playing) {
            MRGetPID(queue, ^(int pid) {
                publish(info, playing, pid);
            });
        });
    });
}

// Notifications arrive in bursts (app, info and playing state change
// together), so coalesce them into a single refresh.
static void scheduleRefresh(void) {
    if (refreshQueued) return;
    refreshQueued = YES;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(80 * NSEC_PER_MSEC)), dispatch_get_main_queue(), ^{
        refreshQueued = NO;
        refresh();
    });
}

static void handleCommand(NSString *line) {
    NSString *command = [line stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];
    if ([command isEqualToString:@"toggle"]) {
        MRSendCommand(kCommandTogglePlayPause, NULL);
    } else if ([command isEqualToString:@"next"]) {
        MRSendCommand(kCommandNextTrack, NULL);
    } else if ([command isEqualToString:@"previous"]) {
        MRSendCommand(kCommandPreviousTrack, NULL);
    } else if ([command hasPrefix:@"seek "] && MRSetElapsedTime) {
        MRSetElapsedTime([[command substringFromIndex:5] doubleValue]);
    }
    scheduleRefresh();
}

static void listenForCommands(void) {
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_UTILITY, 0), ^{
        char *buffer = NULL;
        size_t capacity = 0;
        ssize_t length;
        while ((length = getline(&buffer, &capacity, stdin)) > 0) {
            NSString *line = [[NSString alloc] initWithBytes:buffer length:(NSUInteger)length encoding:NSUTF8StringEncoding];
            if (!line) continue;
            dispatch_async(dispatch_get_main_queue(), ^{ handleCommand(line); });
        }
        free(buffer);
        exit(0);
    });
}

static BOOL loadMediaRemote(void) {
    void *handle = dlopen("/System/Library/PrivateFrameworks/MediaRemote.framework/MediaRemote", RTLD_NOW);
    if (!handle) return NO;
    MRGetNowPlayingInfo = dlsym(handle, "MRMediaRemoteGetNowPlayingInfo");
    MRGetIsPlaying = dlsym(handle, "MRMediaRemoteGetNowPlayingApplicationIsPlaying");
    MRGetPID = dlsym(handle, "MRMediaRemoteGetNowPlayingApplicationPID");
    MRRegisterForNotifications = dlsym(handle, "MRMediaRemoteRegisterForNowPlayingNotifications");
    MRSendCommand = dlsym(handle, "MRMediaRemoteSendCommand");
    MRSetElapsedTime = dlsym(handle, "MRMediaRemoteSetElapsedTime");
    return MRGetNowPlayingInfo && MRGetIsPlaying && MRGetPID && MRRegisterForNotifications && MRSendCommand;
}

void bangs_media_run(void) {
    @autoreleasepool {
        if (!loadMediaRemote()) {
            fprintf(stderr, "MediaRemote is unavailable\n");
            exit(2);
        }

        MRRegisterForNotifications(dispatch_get_main_queue());
        NSArray<NSString *> *names = @[
            @"kMRMediaRemoteNowPlayingInfoDidChangeNotification",
            @"kMRMediaRemoteNowPlayingApplicationDidChangeNotification",
            @"kMRMediaRemoteNowPlayingApplicationIsPlayingDidChangeNotification",
        ];
        for (NSString *name in names) {
            [NSNotificationCenter.defaultCenter addObserverForName:name object:nil queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *note) {
                scheduleRefresh();
            }];
        }

        // Some players skip notifications for seeks; a slow poll keeps the
        // elapsed time honest. Unchanged lines are deduplicated in writeLine.
        [NSTimer scheduledTimerWithTimeInterval:5 repeats:YES block:^(NSTimer *timer) { refresh(); }];

        listenForCommands();
        refresh();
        CFRunLoopRun();
    }
}
