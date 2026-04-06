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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBookFont.js
var require_faBookFont = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBookFont.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "book-font";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e0bf";
    var svgPathData = "M0 96C0 43 43 0 96 0L384 0l32 0c17.7 0 32 14.3 32 32l0 320c0 17.7-14.3 32-32 32l0 64c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0L96 512c-53 0-96-43-96-96L0 96zM64 416c0 17.7 14.3 32 32 32l256 0 0-64L96 384c-17.7 0-32 14.3-32 32zM254.3 72.8c-2.7-5.4-8.3-8.8-14.3-8.8s-11.6 3.4-14.3 8.8l-64 128-32 64c-4 7.9-.7 17.5 7.2 21.5s17.5 .7 21.5-7.2L185.9 224l108.2 0 27.6 55.2c4 7.9 13.6 11.1 21.5 7.2s11.1-13.6 7.2-21.5l-32-64-64-128zM240 115.8L278.1 192l-76.2 0L240 115.8z";
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
    exports.faBookFont = exports.definition;
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

// src/_entries/faBookFont.js
var import_faBookFont = __toESM(require_faBookFont());
var export_faBookFont = import_faBookFont.definition;
export {
  export_faBookFont as faBookFont
};
