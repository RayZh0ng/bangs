import { relativeTime } from "../lib/format";
import { t } from "../lib/i18n";
import { useNow } from "../lib/useNow";
import { useActivities } from "../store/activities";
import { Empty } from "./Empty";
import { BoardIcon, GlyphFor } from "./Icons";

/** Rows other programs put on the notch; see docs/plugins.md. */
export function ActivityPanel() {
  const items = useActivities((state) => state.items);
  const { open } = useActivities.getState();
  const now = useNow(20_000);

  if (!items.length) {
    return (
      <Empty
        icon={<BoardIcon />}
        title={t("还没有别的程序上岛", "Nothing has docked yet")}
        hint={t("往 activities 目录写个 JSON 就会出现在这里", "Drop a JSON in the activities folder to show up here")}
      />
    );
  }

  return (
    <div className="rows rows--snap">
      {items.map((item) => {
        const Row = item.url ? "button" : "div";
        return (
          <Row
            key={item.id}
            className="row"
            title={item.url || item.title}
            onClick={item.url ? () => void open(item) : undefined}
          >
            <span className="row__icon row__icon--glyph">
              <GlyphFor name={item.icon} />
            </span>
            <span className="row__main">
              <span className="row__title">{item.title}</span>
              {item.subtitle && <span className="row__sub">{item.subtitle}</span>}
            </span>
            {typeof item.progress === "number" ? (
              <span className="meter" aria-hidden>
                <i style={{ width: `${Math.round(item.progress * 100)}%` }} />
              </span>
            ) : (
              <span className="row__meta">{relativeTime(item.updatedAt * 1000, now)}</span>
            )}
          </Row>
        );
      })}
    </div>
  );
}
