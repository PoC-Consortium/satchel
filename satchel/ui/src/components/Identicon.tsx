import { Box } from "@mui/material";
import { hashId, idHue } from "../identity";

// A GitHub-style 5×5 symmetric blocky avatar, derived purely from the identity
// pubkey. Same key → same picture, so a maker is recognisable at a glance
// across offers; a different key can't fake it. No image assets, no deps.
export default function Identicon({
  id,
  size = 24,
  title,
}: {
  id: string | null | undefined;
  size?: number;
  title?: string;
}) {
  const hex = id || "";
  const h = hashId(hex);
  const hue = idHue(hex);
  const fg = `hsl(${hue} 58% 60%)`;
  const bg = `hsl(${hue} 24% 22% / 0.55)`;

  // 5 columns with vertical symmetry → fill the left 3 (15 cells) from the hash.
  const cells: boolean[] = [];
  for (let i = 0; i < 15; i++) cells.push(((h >> i) & 1) === 1);
  const filled = (col: number, row: number) => {
    const c = col < 3 ? col : 4 - col; // mirror cols 3,4 onto 1,0
    return cells[c * 5 + row];
  };

  const cell = size / 5;
  const rects: React.ReactNode[] = [];
  for (let col = 0; col < 5; col++) {
    for (let row = 0; row < 5; row++) {
      if (filled(col, row)) {
        rects.push(
          <rect key={`${col}-${row}`} x={col * cell} y={row * cell} width={cell} height={cell} />,
        );
      }
    }
  }

  return (
    <Box
      component="span"
      title={title}
      sx={{
        display: "inline-flex",
        width: size,
        height: size,
        borderRadius: "6px",
        overflow: "hidden",
        bgcolor: bg,
        flex: "none",
      }}
    >
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} fill={fg} aria-hidden>
        {rects}
      </svg>
    </Box>
  );
}
