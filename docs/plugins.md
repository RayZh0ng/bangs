# Docking a row on the notch

Anything running on the same machine can put a line on Bangs. There is no SDK
and nothing to link against: a plugin is whatever writes a small JSON file into
a directory. A loopback HTTP endpoint is there for convenience and writes the
same files, so both roads lead to one place.

A row shows up twice: beside the collapsed notch — the most recently written
one — and as a list under the **上岛 / Board** tab, which only appears once
something has docked.

## The directory

| | |
| --- | --- |
| macOS | `~/Library/Application Support/com.gxlself.bangs/activities/` |
| Windows | `%APPDATA%\com.gxlself.bangs\activities\` |

One file per row. The file name (without `.json`) is the row's id, so writing
the same file again updates the row in place rather than adding another.

```bash
cat > ~/Library/Application\ Support/com.gxlself.bangs/activities/ci.json <<'JSON'
{
  "title": "CI 跑完了",
  "subtitle": "3 个用例挂了",
  "icon": "code",
  "url": "https://github.com/gxlself/bangs/actions",
  "expiresAt": 1789000000
}
JSON
```

Delete the file to take the row down. Bangs notices either within a second.

## The fields

| Field | Type | What it does |
| --- | --- | --- |
| `title` | string, required | The line itself. Beside the notch this is all that is read. |
| `subtitle` | string | Second line in the panel. |
| `id` | string | Defaults to the file name. `A–Z a–z 0–9 - _`, up to 64 characters. |
| `icon` | string | One of `board` `music` `shelf` `code` `clipboard` `folder` `check` `pin`. Anything else draws a neutral dot. |
| `progress` | number | `0`–`1`. Draws a bar, in the panel and beside the notch. |
| `url` | string | `http://` or `https://` only. Clicking the row opens it in the default browser. |
| `expiresAt` | number | Unix seconds. The row leaves on its own and the file is deleted. |

Text is trimmed to 200 characters, and at most 12 rows are shown, newest first.

**A row can carry a link, never a command.** Writing that directory is an
ordinary file write that scripts, hooks and CI runners do casually, so it must
never be a way to get code to run: `file://`, custom app schemes and
`javascript:` are dropped on the way in, and there is no field for a shell
command. Use `expiresAt` for anything that should not outlive the thing it is
reporting on.

## The loopback API

Bangs listens on `127.0.0.1:17650` when the port is free. Every request needs
the token in `api-token`, next to `settings.json` in the config directory
(`~/Library/Application Support/com.gxlself.bangs/api-token`, mode 600). The
token exists to keep a web page in a browser from posting to the port — any
request carrying an `Origin` header is refused outright.

```bash
TOKEN=$(cat ~/Library/Application\ Support/com.gxlself.bangs/api-token)

# Put a row up, or update it
curl -sS -X POST localhost:17650/activity \
  -H "X-Bangs-Token: $TOKEN" \
  -d '{"id":"deploy","title":"发布中","subtitle":"v0.1.4","progress":0.4,"icon":"board"}'

# Read what is on the notch
curl -sS localhost:17650/activities -H "X-Bangs-Token: $TOKEN"

# Take it down
curl -sS -X DELETE localhost:17650/activity/deploy -H "X-Bangs-Token: $TOKEN"
```

```js
// Node, no dependencies.
const token = require("fs").readFileSync(
  `${process.env.HOME}/Library/Application Support/com.gxlself.bangs/api-token`, "utf8").trim();

await fetch("http://127.0.0.1:17650/activity", {
  method: "POST",
  headers: { "X-Bangs-Token": token },
  body: JSON.stringify({ id: "tests", title: "测试通过", icon: "check" }),
});
```

If the port is taken, Bangs says so once on stderr and keeps watching the
directory, which never stops working.

## Notes

- Rows are not notifications: nothing pops open and nothing makes a sound.
  A row waits to be looked at.
- The list is sorted by when each file was last written, so touching a file
  brings its row back to the front.
- A plugin that dies leaves its file behind. `expiresAt` is how a row cleans up
  after itself.
