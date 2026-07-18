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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTentDoublePeak.js
var require_faTentDoublePeak = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTentDoublePeak.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tent-double-peak";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e627";
    var svgPathData = "M182.6 9.4C176.2 2.9 167.3-.5 158.2 0s-17.5 4.9-23.2 12L93.9 63.5c-13.5 16.9-29 31.9-46.3 44.9L12.8 134.4C4.7 140.4 0 149.9 0 160l0 32 576 0 0-32c0-10.1-4.7-19.6-12.8-25.6l-34.7-26.1c-17.3-12.9-32.8-28-46.3-44.9L441 12C435.3 4.9 426.9 .6 417.8 0s-18 2.9-24.4 9.3L374.6 28.1C351.7 51.1 320.5 64 288 64s-63.7-12.9-86.6-35.9L182.6 9.4zM576 224L0 224 0 480c0 17.7 14.3 32 32 32l208 0c0 0 0 0 0 0l16 0c17.7 0 32-14.3 32-32l0-182.1c0-5.5 4.4-9.9 9.9-9.9c3.7 0 7.2 2.1 8.8 5.5L407.2 494.3c5.4 10.8 16.5 17.7 28.6 17.7l28.2 0c0 0 0 0 0 0l80 0c17.7 0 32-14.3 32-32l0-256z";
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
    exports.faTentDoublePeak = exports.definition;
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

// src/_entries/faTentDoublePeak.js
var import_faTentDoublePeak = __toESM(require_faTentDoublePeak());
var export_faTentDoublePeak = import_faTentDoublePeak.definition;
export {
  export_faTentDoublePeak as faTentDoublePeak
};
