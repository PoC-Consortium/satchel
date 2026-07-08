import { createContext, useContext } from "react";
import type { Route } from "../components/Sidebar";

// Lets screens jump between tabs (the old show('coins') / show('swaps')
// cross-links), without prop-drilling the router through every screen. The
// optional second arg deep-links a Settings sub-tab (e.g. navigate("settings",
// "coins") from a "set up coins" empty-state CTA).
export const NavCtx = createContext<(r: Route, tab?: string) => void>(() => {});
export const useNavigate = () => useContext(NavCtx);
