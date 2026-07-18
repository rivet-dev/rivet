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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faEnvelopeDot.js
var require_faEnvelopeDot = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faEnvelopeDot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "envelope-dot";
    var width = 576;
    var height = 512;
    var aliases = ["envelope-badge"];
    var unicode = "e16f";
    var svgPathData = "M496 0a80 80 0 1 1 0 160A80 80 0 1 1 496 0zM48 64l337.1 0c-.7 5.2-1.1 10.6-1.1 16c0 45.6 27.2 84.8 66.3 102.3L275.2 313.6c-11.4 8.5-27 8.5-38.4 0L19.2 150.4C7.1 141.3 0 127.1 0 112C0 85.5 21.5 64 48 64zM294.4 339.2L490.8 191.9c1.7 .1 3.4 .1 5.2 .1c5.4 0 10.8-.4 16-1.1L512 384c0 35.3-28.7 64-64 64L64 448c-35.3 0-64-28.7-64-64L0 176 217.6 339.2c22.8 17.1 54 17.1 76.8 0z";
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
    exports.faEnvelopeDot = exports.definition;
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

// src/_entries/faEnvelopeDot.js
var import_faEnvelopeDot = __toESM(require_faEnvelopeDot());
var export_faEnvelopeDot = import_faEnvelopeDot.definition;
export {
  export_faEnvelopeDot as faEnvelopeDot
};
