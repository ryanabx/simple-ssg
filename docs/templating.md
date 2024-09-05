# Templating

Using templates in `smpl-ssg` is simple! All you need to do is provide a `template.html` file in any directory of your documentation.

> **NOTE:** A nested `template.html` will take priority over a parent folder's `template.html`

An example template could be this:

```html
<!DOCTYPE html>
<html>
    <head>
        <style>
            <!-- insert a style here -->
        </style>
    </head>
    <body>
      <h1>Contents:</h1>
        <!-- {TABLE_OF_CONTENTS} -->
      <hr>
        <!-- {CONTENT} -->
    </body>
</html>
```

This template contains two macros, the `<!-- {TABLE_OF_CONTENTS} -->` macro which provides a simple bulleted list of all the page links for your static site, and `<!-- {CONTENT} -->`, which is where the output of the Markdown and Djot converters goes.