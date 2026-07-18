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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDisplayChartUp.js
var require_faDisplayChartUp = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDisplayChartUp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "display-chart-up";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e5e3";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L512 0c35.3 0 64 28.7 64 64l0 288c0 35.3-28.7 64-64 64l-176 0 10.7 32 69.3 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-256 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l69.3 0L240 416 64 416c-35.3 0-64-28.7-64-64L0 64zm64 0l0 288 448 0 0-288L64 64zM376 96l80 0c13.3 0 24 10.7 24 24l0 80c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-22.1L321 289c-4.5 4.5-10.6 7-17 7s-12.5-2.5-17-7l-63-63-87 87c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9L207 175c9.4-9.4 24.6-9.4 33.9 0l63 63L398.1 144 376 144c-13.3 0-24-10.7-24-24s10.7-24 24-24z";
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
    exports.faDisplayChartUp = exports.definition;
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

// src/_entries/faDisplayChartUp.js
var import_faDisplayChartUp = __toESM(require_faDisplayChartUp());
var export_faDisplayChartUp = import_faDisplayChartUp.definition;
export {
  export_faDisplayChartUp as faDisplayChartUp
};
