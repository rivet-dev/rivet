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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faH5.js
var require_faH5 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faH5.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "h5";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e412";
    var svgPathData = "M64 96c0-17.7-14.3-32-32-32S0 78.3 0 96L0 256 0 416c0 17.7 14.3 32 32 32s32-14.3 32-32l0-128 192 0 0 128c0 17.7 14.3 32 32 32s32-14.3 32-32l0-160 0-160c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 128L64 224 64 96zM432 64c-15.1 0-28.2 10.6-31.3 25.4l-32 152c-2 9.4 .4 19.3 6.5 26.8s15.2 11.8 24.8 11.8l124 0c28.7 0 52 23.3 52 52s-23.3 52-52 52l-67.6 0c-10.3 0-19.5-6.6-22.8-16.4l-3.2-9.7c-5.6-16.8-23.7-25.8-40.5-20.2s-25.8 23.7-20.2 40.5l3.2 9.7c12 35.9 45.6 60.2 83.5 60.2l67.6 0c64.1 0 116-51.9 116-116s-51.9-116-116-116l-84.6 0L458 128l118 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L432 64z";
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
    exports.faH5 = exports.definition;
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

// src/_entries/faH5.js
var import_faH5 = __toESM(require_faH5());
var export_faH5 = import_faH5.definition;
export {
  export_faH5 as faH5
};
