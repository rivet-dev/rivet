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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSnooze.js
var require_faSnooze = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSnooze.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "snooze";
    var width = 448;
    var height = 512;
    var aliases = [128164, "zzz"];
    var unicode = "f880";
    var svgPathData = "M192 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l32 0-57.6 76.8c-7.3 9.7-8.4 22.7-3 33.5s16.5 17.7 28.6 17.7l96 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0 57.6-76.8c7.3-9.7 8.4-22.7 3-33.5S300.1 0 288 0L192 0zM320 224c-17.7 0-32 14.3-32 32s14.3 32 32 32l32 0-57.6 76.8c-7.3 9.7-8.4 22.7-3 33.5s16.5 17.7 28.6 17.7l96 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0 57.6-76.8c7.3-9.7 8.4-22.7 3-33.5S428.1 224 416 224l-96 0zM32 256c-17.7 0-32 14.3-32 32s14.3 32 32 32l91.7 0L7.4 459.5c-7.9 9.5-9.7 22.8-4.4 34.1S19.6 512 32 512l160 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-91.7 0L216.6 308.5c7.9-9.5 9.7-22.8 4.4-34.1s-16.6-18.4-29-18.4L32 256z";
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
    exports.faSnooze = exports.definition;
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

// src/_entries/faZzz.js
var import_faSnooze = __toESM(require_faSnooze());
var export_faZzz = import_faSnooze.definition;
export {
  export_faZzz as faZzz
};
