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

// src/node_modules/@fortawesome/free-solid-svg-icons/faRulerCombined.js
var require_faRulerCombined = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faRulerCombined.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ruler-combined";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f546";
    var svgPathData = "M1 441.7C5.5 463.5 24.8 480 48 480l352 0c26.5 0 48-21.5 48-48l0-96c0-26.5-21.5-48-48-48l-48 0 0 72c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-72-64 0 0 72c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-72-72 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l72 0 0-64-72 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l72 0 0-48c0-26.5-21.5-48-48-48L48 32C21.5 32 0 53.5 0 80L0 432c0 3.3 .3 6.6 1 9.7z";
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
    exports.faRulerCombined = exports.definition;
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

// src/_entries/faRulerCombined.js
var import_faRulerCombined = __toESM(require_faRulerCombined());
var export_faRulerCombined = import_faRulerCombined.definition;
export {
  export_faRulerCombined as faRulerCombined
};
