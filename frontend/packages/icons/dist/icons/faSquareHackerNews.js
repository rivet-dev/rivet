var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/free-brands-svg-icons/faSquareHackerNews.js
var require_faSquareHackerNews = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSquareHackerNews.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "square-hacker-news";
    var width = 448;
    var height = 512;
    var aliases = ["hacker-news-square"];
    var unicode = "f3af";
    var svgPathData = "M448 96c0-35.3-28.7-64-64-64L64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320zM21 229.2s.1-.1 .1-.2 .1-.2 .2-.2c0 .1 0 .3-.1 .4l-.2 0zM239.2 384l-31.4 0 0-102.7-79.8-153.3 37.3 0c41.5 77.7 48.1 95.8 54.1 112 1.6 4.3 3.1 8.5 5.2 13.6 3.2-7 5.1-11.9 7.1-17.3 5.9-15.3 12.8-33.2 53.5-108.3l34.8 0-80.8 155.1 0 100.9z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faSquareHackerNews = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faSquareHackerNews.js
var import_faSquareHackerNews = __toESM(require_faSquareHackerNews());
var export_faSquareHackerNews = import_faSquareHackerNews.definition;
export {
  export_faSquareHackerNews as faSquareHackerNews
};
