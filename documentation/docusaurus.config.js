const path = require('path');

module.exports = {
  plugins: [
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'inx-chronicle',
        path: path.resolve(__dirname, 'docs'),
        routeBasePath: 'chronicle',
        sidebarPath: path.resolve(__dirname, 'sidebars.js'),
        editUrl: 'https://github.com/iotaledger/inx-chronicle/edit/main/documentation',
        remarkPlugins: [require('remark-code-import'), require('remark-import-partial')],
      }
    ],
  ],
  staticDirectories: [path.resolve(__dirname, 'static')],
};
