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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faH2.js
var require_faH2 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faH2.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "h2";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f314";
    var svgPathData = "M64 96c0-17.7-14.3-32-32-32S0 78.3 0 96L0 256 0 416c0 17.7 14.3 32 32 32s32-14.3 32-32l0-128 192 0 0 128c0 17.7 14.3 32 32 32s32-14.3 32-32l0-160 0-160c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 128L64 224 64 96zm385.9 47.4c11.6-9.9 26.4-15.4 41.7-15.4l4.5 0c35.3 0 64 28.7 64 64l0 5.8c0 17.9-7.5 35.1-20.8 47.2L378.4 392.4c-9.7 8.9-13 22.9-8.2 35.2S386.8 448 400 448l208 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-125.7 0 100.2-91.9c26.4-24.2 41.5-58.5 41.5-94.4l0-5.8c0-70.7-57.3-128-128-128l-4.5 0c-30.6 0-60.1 10.9-83.3 30.8l-29 24.9c-13.4 11.5-15 31.7-3.5 45.1s31.7 15 45.1 3.5l29-24.9z";
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
    exports.faH2 = exports.definition;
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

// src/_entries/faH2.js
var import_faH2 = __toESM(require_faH2());
var export_faH2 = import_faH2.definition;
export {
  export_faH2 as faH2
};
