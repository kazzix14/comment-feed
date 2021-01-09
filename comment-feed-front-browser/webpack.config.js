const path = require('path');
const webpack = require('webpack');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');

const distPath = path.resolve(__dirname, "dist");
module.exports = (env, argv) => {
  return {
    devServer: {
      contentBase: distPath,
      compress: argv.mode === 'production',
      port: 8000
    },
    entry: './bootstrap.js',
    output: {
      path: distPath,
      filename: "comment-feed.js",
      webassemblyModuleFilename: "comment-feed.wasm"
    },
    module: {
      rules: [
        {
          test: /\.s[ac]ss$/i,
          use: [
            'style-loader',
            'css-loader',
            'sass-loader',
          ],
        },
        {
          test: /\.css$/i,
          use: [
            'style-loader',
            'css-loader',
          ],
        },
        {
          test: /\.(eot|svg|ttf|woff|woff2|png|jpe?g|gif)$/,
          loader: 'file-loader?name=semantic/dist/themes/default/assets/fonts/[name].[ext]'
        },
      ],
    },
    plugins: [
      new CopyWebpackPlugin([
        { from: './static', to: distPath }
      ]),
      new WasmPackPlugin({
        crateDirectory: ".",
        extraArgs: "--no-typescript",
      }),
      new webpack.ProvidePlugin({
        $: 'jquery',
        jQuery: 'jquery'
      })
    ],
    watch: argv.mode !== 'production'
  };
};
