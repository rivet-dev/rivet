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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChartLineUpDown.js
var require_faChartLineUpDown = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChartLineUpDown.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chart-line-up-down";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5d7";
    var svgPathData = "M64 64c0-17.7-14.3-32-32-32S0 46.3 0 64L0 400c0 44.2 35.8 80 80 80l400 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L80 416c-8.8 0-16-7.2-16-16L64 64zm313.4 57.4L288 210.7l-57.4-57.4c-12.5-12.5-32.8-12.5-45.3 0l-80 80c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L208 221.3l57.4 57.4c12.5 12.5 32.8 12.5 45.3 0l112-112 18.7 18.7c4.2 4.2 10 6.6 16 6.6c12.5 0 22.6-10.1 22.6-22.6l0-73.2 0-.3L480 80c0-8.8-7.2-16-16-16l-89.4 0C362.1 64 352 74.1 352 86.6c0 6 2.4 11.8 6.6 16l18.7 18.7zm64 141.3l-18.7 18.7-34.7-34.7-45.3 45.3 34.7 34.7-18.7 18.7c-4.2 4.2-6.6 10-6.6 16c0 12.5 10.1 22.6 22.6 22.6l89.4 0c8.8 0 16-7.2 16-16l0-89.4c0-12.5-10.1-22.6-22.6-22.6c-6 0-11.8 2.4-16 6.6z";
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
    exports.faChartLineUpDown = exports.definition;
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

// src/_entries/faChartLineUpDown.js
var import_faChartLineUpDown = __toESM(require_faChartLineUpDown());
var export_faChartLineUpDown = import_faChartLineUpDown.definition;
export {
  export_faChartLineUpDown as faChartLineUpDown
};
