import { createContext, useContext } from "react";
import type { Route } from "../components/Sidebar";

// Lets screens jump between tabs (the old show('coins') / show('swaps')
// cross-links), without prop-drilling the router through every screen.
export const NavCtx = createContext<(r: Route) => void>(() => {});
export const useNavigate = () => useContext(NavCtx);
