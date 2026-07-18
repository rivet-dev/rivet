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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSignalStrong.js
var require_faSignalStrong = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSignalStrong.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "signal-strong";
    var width = 640;
    var height = 512;
    var aliases = ["signal-4"];
    var unicode = "f68f";
    var svgPathData = "M480 128c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 352c0 17.7 14.3 32 32 32s32-14.3 32-32l0-352zM352 224c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 256c0 17.7 14.3 32 32 32s32-14.3 32-32l0-256zM192 288c-17.7 0-32 14.3-32 32l0 160c0 17.7 14.3 32 32 32s32-14.3 32-32l0-160c0-17.7-14.3-32-32-32zM96 416c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 64c0 17.7 14.3 32 32 32s32-14.3 32-32l0-64z";
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
    exports.faSignalStrong = exports.definition;
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

// src/_entries/faSignal4.js
var import_faSignalStrong = __toESM(require_faSignalStrong());
var export_faSignal4 = import_faSignalStrong.definition;
export {
  export_faSignal4 as faSignal4
};
