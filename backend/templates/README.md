# Widget template for Wendy Widget Framework

### Directory structure

```
<widget-id>/
├── assets/
│   └── widget.png          # Static media and icons used by the widget
├── content/                # Main widget view rendered on the widget window
│   ├── index.html          # Markup entry point for the widget view
│   ├── jsconfig.json
│   ├── script.js           # Widget logic and state, exposes host bridge
│   └── styles.css          # Stylesheet for the widget view
├── editor/                 # Settings/configuration UI for customizing the widget
│   ├── index.html          # Markup entry point for the widget editor
│   ├── jsconfig.json
│   ├── script.js           # Settings form logic
│   └── styles.css          # Stylesheet for the editor view
├── types/                  # Ambient TypeScript declarations (editor IntelliSense only)
│   ├── content.d.ts        # APIs and message types exclusive to content/
│   ├── editor.d.ts         # APIs and message types exclusive to editor/
│   └── common.d.ts         # Common types used by both views
├── base.jsconfig.json      # Shared compiler options extended by subfolder jsconfigs
├── manifest.json           # Widget metadata, permissions, etc.
├── preview.png             # Thumbnail shown in the widget gallery/picker
└── README.md               # Setup instructions and widget documentation
```

---

### References

- [Repository](https://github.com/CyanFroste/wendy)
