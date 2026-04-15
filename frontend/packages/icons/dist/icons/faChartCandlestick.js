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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChartCandlestick.js
var require_faChartCandlestick = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChartCandlestick.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chart-candlestick";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0e2";
    var svgPathData = "M64 64c0-17.7-14.3-32-32-32S0 46.3 0 64L0 400c0 44.2 35.8 80 80 80l400 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L80 416c-8.8 0-16-7.2-16-16L64 64zM320 48c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 48c-17.7 0-32 14.3-32 32l0 96c0 17.7 14.3 32 32 32l0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48c17.7 0 32-14.3 32-32l0-96c0-17.7-14.3-32-32-32l0-48zm64 176l0 64c0 17.7 14.3 32 32 32l0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48c17.7 0 32-14.3 32-32l0-64c0-17.7-14.3-32-32-32l0-48c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 48c-17.7 0-32 14.3-32 32zM160 128c-17.7 0-32 14.3-32 32l0 128c0 17.7 14.3 32 32 32l0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48c17.7 0 32-14.3 32-32l0-128c0-17.7-14.3-32-32-32l0-48c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 48z";
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
    exports.faChartCandlestick = exports.definition;
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

// src/_entries/faChartCandlestick.js
var import_faChartCandlestick = __toESM(require_faChartCandlestick());
var export_faChartCandlestick = import_faChartCandlestick.definition;
export {
  export_faChartCandlestick as faChartCandlestick
};
