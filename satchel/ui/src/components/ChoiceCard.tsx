import { Box, Paper, Typography } from "@mui/material";
import type { ReactNode } from "react";
import { C } from "../theme";

// The selectable option card used across the wizard / coin setup (seed mode,
// encryption, funding wallet) — the old `.choice` block as a component.
export default function ChoiceCard({
  title,
  desc,
  selected,
  disabled,
  badge,
  onClick,
}: {
  title: ReactNode;
  desc: ReactNode;
  selected?: boolean;
  disabled?: boolean;
  badge?: string;
  onClick?: () => void;
}) {
  return (
    <Paper
      variant="outlined"
      onClick={disabled ? undefined : onClick}
      sx={{
        flex: 1,
        p: 1.75,
        cursor: disabled ? "not-allowed" : "pointer",
        opacity: disabled ? 0.45 : 1,
        bgcolor: selected ? C.raised : "background.default",
        borderColor: selected ? "primary.main" : "divider",
        transition: "border-color .15s ease",
        "&:hover": disabled ? {} : { borderColor: "primary.main" },
      }}
    >
      <Typography sx={{ fontWeight: 600, mb: 0.5, display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <span>{title}</span>
        {badge && (
          <Box
            component="span"
            sx={{
              fontSize: 10,
              textTransform: "uppercase",
              letterSpacing: "0.08em",
              color: "text.secondary",
              border: "1px solid",
              borderColor: "divider",
              borderRadius: 1,
              px: 0.75,
            }}
          >
            {badge}
          </Box>
        )}
      </Typography>
      <Typography sx={{ color: "text.secondary", fontSize: 12 }}>{desc}</Typography>
    </Paper>
  );
}
