# PDFMaker

A fast, lightweight HTML to PDF converter built with Rust. Convert HTML documents with CSS styling into high-quality PDF files.

<img width="680" alt="image" src="https://github.com/user-attachments/assets/4714ba84-454a-47b0-a3cb-8292bd53bac7" />

## PDF
<img width="680" height="800" alt="image" src="https://github.com/user-attachments/assets/88967375-6e58-4bf1-8da9-687a2e63b4c1" />

<img width="680" height="800" alt="image" src="https://github.com/user-attachments/assets/8e689461-486d-44ff-ab8a-5d4bb142a3eb" />

<img width="680" height="800" alt="image" src="https://github.com/user-attachments/assets/192eda84-a985-432c-9337-01e325c641d0" />

## Features

- **HTML to PDF Conversion** - Convert HTML documents with common CSS support
- **Multiple Paper Sizes** - Support for A4, A3, Letter, and Legal paper sizes
- **CSS Styling** - Support for modern CSS including Flexbox, Grid, and more
- **Page Breaks** - Control pagination with `break-before: page` and `break-after: page`
- **Custom Fonts** - Support for various font families and styles
- **Images & SVG** - Embed images and SVG graphics in your PDFs
- **Tables** - Full table support with borders, backgrounds, and cell styling 

## Installation

### Android App

Download PDFMaker from the Google Play Store:

[**Download on Google Play**](https://play.google.com/store/apps/details?id=com.pdfmaker.johnkenedy)

### Paper Sizes

| Size | Dimensions | Points |
|------|------------|--------|
| A4 (default) | 210mm × 297mm | 595 × 842 |
| A3 | 297mm × 420mm | 842 × 1191 |
| Letter | 8.5in × 11in | 612 × 792 |
| Legal | 8.5in × 14in | 612 × 1008 |

## Supported CSS Features

- **Layout**: `display` (block, inline, inline-block, flex, grid, table)
- **Box Model**: `margin`, `padding`, `border`, `width`, `height`
- **Flexbox**: `flex-direction`, `justify-content`, `align-items`, `flex-wrap`, `gap`
- **Grid**: `grid-template-columns`, `grid-template-rows`, `grid-gap`
- **Typography**: `font-family`, `font-size`, `font-weight`, `font-style`, `line-height`, `text-align`, `text-decoration`
- **Colors**: `color`, `background-color`, `opacity`, RGBA support
- **Positioning**: `position` (static, relative, absolute, fixed), `top`, `right`, `bottom`, `left`
- **Page Breaks**: `break-before`, `break-after`, `page-break-before`, `page-break-after`
- **And more**: `transform`, `border-radius`, `box-shadow`, `list-style-type`, etc.

## Example HTML

```html
<!DOCTYPE html>
<html>
<head>
    <style>
        body {
            font-family: Arial, sans-serif;
            margin: 40px;
        }
        h1 {
            color: #333;
            border-bottom: 2px solid #007bff;
        }
        .page-break {
            break-before: page;
        }
    </style>
</head>
<body>
    <h1>My Document</h1>
    <p>This is the first page content.</p>

    <div class="page-break">
        <h1>Second Page</h1>
        <p>This content appears on a new page.</p>
    </div>
</body>
</html>
```

## Links

- [Google Play Store](https://play.google.com/store/apps/details?id=com.pdfmaker.johnkenedy)
