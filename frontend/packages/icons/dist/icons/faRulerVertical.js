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

// src/node_modules/@fortawesome/free-solid-svg-icons/faRulerVertical.js
var require_faRulerVertical = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faRulerVertical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ruler-vertical";
    var width = 256;
    var height = 512;
    var aliases = [];
    var unicode = "f548";
    var svgPathData = "M0 16C0-10.5 21.5-32 48-32l160 0c26.5 0 48 21.5 48 48l0 24-104 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l104 0 0 48-72 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l72 0 0 48-104 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l104 0 0 48-72 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l72 0 0 48-104 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l104 0 0 24c0 26.5-21.5 48-48 48L48 544c-26.5 0-48-21.5-48-48L0 16z";
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
    exports.faRulerVertical = exports.definition;
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

// src/_entries/faRulerVertical.js
var import_faRulerVertical = __toESM(require_faRulerVertical());
var export_faRulerVertical = import_faRulerVertical.definition;
export {
  export_faRulerVertical as faRulerVertical
};
