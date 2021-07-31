const autoprefixer = require('autoprefixer');
const esbuild = require('esbuild');
const postCssPlugin = require('esbuild-plugin-postcss2');
const tailwindcss = require('tailwindcss');

esbuild.build({
  entryPoints: ['app.jsx'],
  bundle: true,
  minify: true,
  sourcemap: true,
  target: 'es2017',
  outfile: 'out.js',
  plugins: [
    postCssPlugin.default({
      plugins: [
        autoprefixer,
        tailwindcss,
      ]
    }),
  ],
}).catch(() => process.exit(1))
