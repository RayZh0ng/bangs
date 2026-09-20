import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";

import { t } from "../lib/i18n";
import { useTodos } from "../store/todos";
import { CheckIcon, PlusIcon } from "./Icons";

/** How far apart the characters of a line start coming loose. */
const STAGGER_MS = 220;

/**
 * The one panel you type into. The notch never takes focus on its own, so the
 * field asks for the keyboard when it is focused and hands it straight back
 * (see `capture_keyboard` and src/store/todos.ts). A line that is done with is
 * not kept around: it comes apart and blows off the list.
 */
export function TodoPanel() {
  const items = useTodos((state) => state.items);
  const dusting = useTodos((state) => state.dusting);
  const typing = useTodos((state) => state.typing);
  const { add, snap, setTyping } = useTodos.getState();
  const [draft, setDraft] = useState("");
  const field = useRef<HTMLInputElement>(null);

  // The panel can be taken away mid-word — an agent alert switches tabs, the
  // notch collapses — and a field that never sees its own blur would leave
  // the keyboard with a window that is no longer showing it.
  useEffect(() => () => useTodos.getState().setTyping(false), []);

  const submit = () => {
    if (!draft.trim()) return;
    void add(draft);
    setDraft("");
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      submit();
    } else if (event.key === "Escape") {
      setDraft("");
      field.current?.blur();
    }
  };

  const left = items.filter((item) => !dusting.includes(item.id)).length;

  return (
    <div className="todo">
      <div className={`todo__new${typing ? " is-typing" : ""}`} onClick={() => field.current?.focus()}>
        <PlusIcon width={13} height={13} />
        <input
          ref={field}
          className="todo__input"
          value={draft}
          placeholder={t("加一件事，回车记下", "Add something, press return")}
          spellCheck={false}
          autoComplete="off"
          onChange={(event) => setDraft(event.target.value)}
          onFocus={() => setTyping(true)}
          onBlur={() => setTyping(false)}
          onKeyDown={onKeyDown}
        />
      </div>

      {items.length ? (
        <div className="rows rows--snap">
          {items.map((item) => {
            const dust = dusting.includes(item.id);
            return (
              <div
                key={item.id}
                className={`row todo__row${dust ? " is-dust" : ""}`}
                title={item.text}
                onClick={() => snap(item.id)}
              >
                <span className="todo__box">{dust && <CheckIcon width={11} height={11} />}</span>
                <span className="row__main">
                  {dust ? (
                    <Dust text={item.text} />
                  ) : (
                    <span className="row__title row__title--plain">{item.text}</span>
                  )}
                </span>
              </div>
            );
          })}
        </div>
      ) : (
        <div className="todo__blank">{t("这里还空着", "Nothing on the list")}</div>
      )}

      <div className="shelf__footer">
        <span>
          {left
            ? t(`还有 ${left} 件 · 点一下就散了`, `${left} to go · click one and it's dust`)
            : t("都做完了", "All done")}
        </span>
      </div>
    </div>
  );
}

/**
 * A line coming apart, character by character. The offsets are worked out from
 * each character's place in the line rather than drawn at random, so the same
 * line always blows away the same way and a re-render never restarts it.
 */
function Dust({ text }: { text: string }) {
  const characters = [...text];
  const last = Math.max(characters.length - 1, 1);
  return (
    <span className="row__title row__title--plain" aria-label={text}>
      {characters.map((character, index) => (
        <span
          key={index}
          className="todo__dust"
          style={
            {
              "--delay": `${Math.round((index / last) * STAGGER_MS)}ms`,
              "--drift": `${12 + ((index * 7) % 14)}px`,
              "--lift": `${-5 - ((index * 5) % 12)}px`,
              "--spin": `${((index * 11) % 26) - 13}deg`,
            } as CSSProperties
          }
        >
          {character === " " ? " " : character}
        </span>
      ))}
    </span>
  );
}
