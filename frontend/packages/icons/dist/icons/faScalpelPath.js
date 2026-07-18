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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faScalpelLineDashed.js
var require_faScalpelLineDashed = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faScalpelLineDashed.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "scalpel-line-dashed";
    var width = 576;
    var height = 512;
    var aliases = ["scalpel-path"];
    var unicode = "f61e";
    var svgPathData = "M441.4 0c-20.8 0-40.6 9.2-54 25.1L187.8 261.7c-4 4.8-4.9 11.4-2.3 17.1s8.3 9.3 14.5 9.3l132.6 0c14.1 0 27.6-6.2 36.7-17L494.4 122.7C505.8 109.2 512 92.2 512 74.6l0-4C512 31.6 480.4 0 441.4 0zM164.7 324.7l-160 160c-4.6 4.6-5.9 11.5-3.5 17.4s8.3 9.9 14.8 9.9c18.1 0 84.1-8.4 145.9-33.1C222.5 454.6 288 410.9 288 336l0-16-16 0-96 0-6.6 0-4.7 4.7zM304 480l-64 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l64 0c8.8 0 16-7.2 16-16s-7.2-16-16-16zm64 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l64 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-64 0zm128 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l64 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-64 0z";
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
    exports.faScalpelLineDashed = exports.definition;
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

// src/_entries/faScalpelPath.js
var import_faScalpelLineDashed = __toESM(require_faScalpelLineDashed());
var export_faScalpelPath = import_faScalpelLineDashed.definition;
export {
  export_faScalpelPath as faScalpelPath
};
