const HtmlWebpackPlugin = require("html-webpack-plugin");

module.exports = {
  experiments: {
    asyncWebAssembly: true,
  },

  entry: "./index.js",
  output: {
    path: __dirname + "/dist",
    filename: "index_bundle.js",
  },
  plugins: [new HtmlWebpackPlugin()],
};
