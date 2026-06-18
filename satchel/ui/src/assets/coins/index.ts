// Real per-coin logo assets, keyed by registry coin id. btc.svg is the
// canonical orange Bitcoin mark (MIT, Jonas Schnelli); btcx.svg is the
// official "Bitcoin PoCX" coin mark with no wordmark, from pocx_marketing.
// CoinGlyph falls back to the generated text glyph for any id not listed here.
import btc from "./btc.svg";
import btcx from "./btcx.svg";

export const COIN_ICON: Record<string, string> = { btc, btcx };
