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

// src/node_modules/@fortawesome/free-solid-svg-icons/faEarListen.js
var require_faEarListen = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faEarListen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ear-listen";
    var width = 512;
    var height = 512;
    var aliases = ["assistive-listening-systems"];
    var unicode = "f2a2";
    var svgPathData = "M379.6 .3c-8.9 1.2-17.2 6.2-22.4 14.1-6.8 10.4-7 24.1-.4 34.6 5.6 8.9 15.5 12.7 23.8 18.5 7.5 5.2 17.6 13.2 27.7 24.2 20 21.7 39.5 54.3 39.5 100.3 0 17.7 14.3 32 32 32s32-14.3 32-32c0-66-28.5-113.4-56.5-143.7-13.9-15.1-27.8-26.1-38.3-33.3-11-7.6-23.5-16.6-37.6-14.7zM240 128c-57.6 0-105.1 43.6-111.3 99.5-1.9 17.6-17.8 30.2-35.3 28.3s-30.2-17.8-28.3-35.3c9.7-88 84.3-156.5 174.9-156.5 97.2 0 176 78.8 176 176 0 46-17.7 87.9-46.6 119.3-12 13-17.4 24.8-17.4 34.7l0 6.1c0 61.9-50.1 112-112 112-17.7 0-32-14.3-32-32s14.3-32 32-32c26.5 0 48-21.5 48-48l0-6.1c0-32.9 17.4-59.6 34.4-78 18.4-20 29.6-46.6 29.6-75.9 0-61.9-50.1-112-112-112zM0 480a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm160-96a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM86.6 361.4c-12.5-12.5-32.8-12.5-45.3 0s-12.5 32.8 0 45.3l64 64c12.5 12.5 32.8 12.5 45.3 0s12.5-32.8 0-45.3l-64-64zM240 208c-17.7 0-32 14.3-32 32 0 13.3-10.7 24-24 24s-24-10.7-24-24c0-44.2 35.8-80 80-80s80 35.8 80 80c0 13.3-10.7 24-24 24s-24-10.7-24-24c0-17.7-14.3-32-32-32z";
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
    exports.faEarListen = exports.definition;
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

// src/_entries/faEarListen.js
var import_faEarListen = __toESM(require_faEarListen());
var export_faEarListen = import_faEarListen.definition;
export {
  export_faEarListen as faEarListen
};
