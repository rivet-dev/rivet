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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLeftLongToLine.js
var require_faLeftLongToLine = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLeftLongToLine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "left-long-to-line";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e41e";
    var svgPathData = "M0 96L0 416c0 17.7 14.3 32 32 32s32-14.3 32-32L64 96c0-17.7-14.3-32-32-32S0 78.3 0 96zM279.5 409.4c7 6.6 17.2 8.4 26 4.6s14.5-12.5 14.5-22l0-72 288 0c17.7 0 32-14.3 32-32l0-64c0-17.7-14.3-32-32-32l-288 0 0-72c0-9.6-5.7-18.2-14.5-22s-19-2-26 4.6l-144 136c-4.8 4.5-7.5 10.8-7.5 17.4s2.7 12.9 7.5 17.4l144 136z";
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
    exports.faLeftLongToLine = exports.definition;
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

// src/_entries/faLeftLongToLine.js
var import_faLeftLongToLine = __toESM(require_faLeftLongToLine());
var export_faLeftLongToLine = import_faLeftLongToLine.definition;
export {
  export_faLeftLongToLine as faLeftLongToLine
};
