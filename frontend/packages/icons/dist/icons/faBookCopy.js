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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBookCopy.js
var require_faBookCopy = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBookCopy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "book-copy";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e0be";
    var svgPathData = "M160 96L96 96C43 96 0 139 0 192L0 416c0 53 43 96 96 96l224 0 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0-32 0L96 448c-17.7 0-32-14.3-32-32s14.3-32 32-32l81.1 0c-10.9-18.8-17.1-40.7-17.1-64l0-224zM320 416l32 0 160 0 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l0-64c17.7 0 32-14.3 32-32l0-224c0-17.7-14.3-32-32-32L512 0 288 0c-53 0-96 43-96 96l0 224c0 24.6 9.2 47 24.4 64c17.6 19.6 43.1 32 71.6 32l32 0zm-64-96c0-17.7 14.3-32 32-32l96 0 96 0 0 64-96 0-96 0c-17.7 0-32-14.3-32-32z";
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
    exports.faBookCopy = exports.definition;
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

// src/_entries/faBookCopy.js
var import_faBookCopy = __toESM(require_faBookCopy());
var export_faBookCopy = import_faBookCopy.definition;
export {
  export_faBookCopy as faBookCopy
};
