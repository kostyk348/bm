# bm — закладки с тегами

Поиск закладок по X-Query, fzf-выбор, открытие через xdg-open. Запись — атомарный
`.eml` (eml-tag), поиск — header-only scan, без индекса и демонов.

```bash
cargo build --release

bm init ~/.bm
bm add ~/.bm https://github.com/kostyk348/mime-os --tag github --tag mime-os --title "MIME-OS"
bm find ~/.bm 'X-Tag == "vita"'
bm ls ~/.bm
bm tags ~/.bm
bm open ~/.bm 'X-Tag == "github"'    # xdg-open первого результата
bm pick ~/.bm 'X-Tag == "rust"'      # fzf-выбор
```

## Свойства

- Один URL = одна запись (`bm_<sha256 url>`); повторный `add` **сливает** теги
- X-Query: `'X-Tag == "a" AND X-Tag == "b" AND X-Timestamp >= 1700000000'`
- Corruption-proof: header-only scan, tmp+rename атомарность, одна битая запись не рушит остальные
- Построен на [emlbox](https://github.com/kostyk348/mime-os) (MIME-OS)

## Тесты

```bash
cargo test
```