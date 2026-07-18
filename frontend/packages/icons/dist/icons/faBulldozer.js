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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBulldozer.js
var require_faBulldozer = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBulldozer.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bulldozer";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e655";
    var svgPathData = "M96 96l0 96 124.2 0-48-96L96 96zM229.5 67.4L291.8 192l28.2 0 0-64c0-17.7 14.3-32 32-32s32 14.3 32 32l0 64 32 0c17.7 0 32 14.3 32 32l0 64 64 0 0-41.6c0-21.8 7.4-42.9 21-60L583 124c11-13.8 31.2-16 45-5s16 31.2 5 45l-50 62.5c-4.5 5.7-7 12.7-7 20l0 73.6 0 69.5c0 8.5 3.4 16.6 9.4 22.6l45.3 45.3c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0l-45.3-45.3c-18-18-28.1-42.4-28.1-67.9l0-37.5-42.8 0c6.9 14.5 10.8 30.8 10.8 48c0 61.9-50.1 112-112 112l-256 0C50.1 512 0 461.9 0 400c0-30.5 12.2-58.2 32-78.4c0-.5 0-1 0-1.6l0-64 0-32L32 80c0-26.5 21.5-48 48-48l92.2 0c24.2 0 46.4 13.7 57.2 35.4zM112 352c-26.5 0-48 21.5-48 48s21.5 48 48 48l256 0c26.5 0 48-21.5 48-48s-21.5-48-48-48l-256 0zm8 24a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm216 24a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm-56-24a24 24 0 1 1 0 48 24 24 0 1 1 0-48zM176 400a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faBulldozer = exports.definition;
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

// src/_entries/faBulldozer.js
var import_faBulldozer = __toESM(require_faBulldozer());
var export_faBulldozer = import_faBulldozer.definition;
export {
  export_faBulldozer as faBulldozer
};
