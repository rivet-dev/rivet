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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBoxOpenFull.js
var require_faBoxOpenFull = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBoxOpenFull.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "box-open-full";
    var width = 640;
    var height = 512;
    var aliases = ["box-full"];
    var unicode = "f49c";
    var svgPathData = "M508.9 64.9l-27.5 73.4L320 158.5l-7.2-.9c4.6-8.8 7.2-18.9 7.2-29.6c0-35.3-28.7-64-64-64s-64 28.7-64 64c0 5.1 .6 10 1.7 14.7l-65.5-8.2c-.1-2.2-.2-4.3-.2-6.5C128 57.3 185.3 0 256 0c69.6 0 126.2 55.6 128 124.7l35.1-93.6c9.3-24.8 37-37.4 61.8-28.1s37.4 37 28.1 61.8zM576 338.3l0 83.4c0 14.7-10 27.5-24.2 31L335.5 506.8c-10.2 2.5-20.9 2.5-31 0L88.2 452.8C74 449.2 64 436.4 64 421.7l0-83.4 127.6 36.5c27.8 8 57.6-3.8 72.5-28.6L320 252.9l55.9 93.2c14.9 24.8 44.6 36.6 72.5 28.6L576 338.3zM58.9 168.8c3-6.1 9.6-9.6 16.3-8.7L320 190.7l244.8-30.6c6.7-.8 13.3 2.7 16.3 8.7l41.7 83.4c9 17.9-.6 39.6-19.8 45.1L439.6 344c-13.9 4-28.8-1.9-36.2-14.3L320 190.7l-83.4 139c-7.4 12.4-22.3 18.3-36.2 14.3L37.1 297.3c-19.3-5.5-28.8-27.2-19.8-45.1l41.7-83.4z";
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
    exports.faBoxOpenFull = exports.definition;
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

// src/_entries/faBoxOpenFull.js
var import_faBoxOpenFull = __toESM(require_faBoxOpenFull());
var export_faBoxOpenFull = import_faBoxOpenFull.definition;
export {
  export_faBoxOpenFull as faBoxOpenFull
};
