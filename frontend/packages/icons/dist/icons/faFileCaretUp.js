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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPageCaretUp.js
var require_faPageCaretUp = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPageCaretUp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "page-caret-up";
    var width = 384;
    var height = 512;
    var aliases = ["file-caret-up"];
    var unicode = "e42a";
    var svgPathData = "M320 512c35.3 0 64-28.7 64-64l0-309.5c0-17-6.7-33.3-18.7-45.3L290.7 18.7C278.7 6.7 262.5 0 245.5 0L64 0C28.7 0 0 28.7 0 64L0 448c0 35.3 28.7 64 64 64l256 0zM192 192c4.5 0 8.8 1.9 11.8 5.2l75.7 82.6c14.1 15.4 3.2 40.2-17.7 40.2l-139.6 0c-20.9 0-31.8-24.8-17.7-40.2l75.7-82.6c3-3.3 7.3-5.2 11.8-5.2z";
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
    exports.faPageCaretUp = exports.definition;
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

// src/_entries/faFileCaretUp.js
var import_faPageCaretUp = __toESM(require_faPageCaretUp());
var export_faFileCaretUp = import_faPageCaretUp.definition;
export {
  export_faFileCaretUp as faFileCaretUp
};
