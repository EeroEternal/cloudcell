# Cloudcell console

Operator UI for cloudcell. Copied from console-kit; product pages live in `src/pages/`.

```bash
npm install
npm run dev          # proxies /api to http://127.0.0.1:8080
npm run lint
bash ../scripts/check_ui_stack.sh && bash ../scripts/check_admin_nav.sh
```

Production: Cloudflare Pages, root `admin/`, `VITE_API_BASE=https://api.cloudcell.dev`. See [`docs/deploy.md`](../docs/deploy.md).

Sidebar = `src/lib/nav.ts` + `pages/*.tsx` + `App.tsx` route. Do not invent a second menu.

Kit copy list: [`KIT.md`](KIT.md).
