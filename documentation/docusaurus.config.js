const path = require('path');

module.exports = {
  plugins: [
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'docs-template',  // Usually your repository's name, in this case docs-template
        path: path.resolve(__dirname, 'docs'),
        routeBasePath: 'docs-template', // Usually your repository's name, in this case docs-template
        sidebarPath: path.resolve(__dirname, 'sidebars.js'),
        editUrl: 'https://YOURREPOSITORYURL/edit/YOURDESIREDBRANCHNAME/documentation',// Example: https://github.com/iotacommunity/docs-template/edit/production/documentation
        remarkPlugins: [require('remark-code-import'), require('remark-import-partial')],// You can add any remark or rehype extensions you may need here 
      }
    ],
  ],
  staticDirectories: [path.resolve(__dirname, 'static')],
};
