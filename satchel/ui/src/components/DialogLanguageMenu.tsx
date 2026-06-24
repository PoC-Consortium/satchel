import { Box } from "@mui/material";
import LanguageMenu from "./LanguageMenu";

// A compact language switcher pinned to the top-right corner of an onboarding
// dialog. During first-run the header's own picker sits behind the dialog
// backdrop, so each gate dialog carries this one instead — reachable, and
// clearly part of the dialog rather than a stray globe over the backdrop.
//
// Drop it as a child of a <Dialog> whose Paper is position:relative
// (slotProps={{ paper: { sx: { position: "relative" } } }}); it anchors to that
// Paper. The dropdown renders above the dialog backdrop (menuZIndex).
export default function DialogLanguageMenu() {
  return (
    <Box sx={{ position: "absolute", top: 6, right: 6, zIndex: 1 }}>
      <LanguageMenu iconOnly menuZIndex={1500} />
    </Box>
  );
}
