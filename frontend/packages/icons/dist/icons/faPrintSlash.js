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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPrintSlash.js
var require_faPrintSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPrintSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "print-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f686";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7L522.2 384l21.8 0c17.7 0 32-14.3 32-32l0-96c0-35.3-28.7-64-64-64l-234.8 0L192 125.2 192 64l226.7 0L448 93.3l0 66.7 64 0 0-66.7c0-17-6.7-33.3-18.7-45.3L464 18.7C452 6.7 435.7 0 418.7 0L192 0c-35.3 0-64 28.7-64 64l0 11L38.8 5.1zm459 483.1l-51-40.2L192 448l0-64 0-16 0-16 132.9 0L122.2 192.3C89.6 195.2 64 222.6 64 256l0 96c0 17.7 14.3 32 32 32l32 0 0 64c0 35.3 28.7 64 64 64l256 0c20.1 0 38.1-9.3 49.8-23.8zM496 248a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faPrintSlash = exports.definition;
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

// src/_entries/faPrintSlash.js
var import_faPrintSlash = __toESM(require_faPrintSlash());
var export_faPrintSlash = import_faPrintSlash.definition;
export {
  export_faPrintSlash as faPrintSlash
};
