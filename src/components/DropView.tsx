import { t } from "../lib/i18n";
import type { Size } from "../lib/layout";
import { useNotch } from "../store/notch";
import { CheckIcon, ShelfIcon } from "./Icons";

export function DropView({ base, success }: { base: Size; success: boolean }) {
  const successText = useNotch((s) => s.successText);

  return (
    <div className="drop" style={{ paddingTop: base.height }}>
      <div className={`drop__zone${success ? " is-success" : ""}`}>
        {success ? <CheckIcon width={18} height={18} /> : <ShelfIcon width={18} height={18} />}
        <span>{success ? successText : t("松手放进暂存架", "Drop to keep it here")}</span>
      </div>
    </div>
  );
}
