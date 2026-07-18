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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDiamonds4.js
var require_faDiamonds4 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDiamonds4.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "diamonds-4";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e68b";
    var svgPathData = "M227.7 11.7c15.6-15.6 40.9-15.6 56.6 0l74.1 74.1c12.5 12.5 12.5 32.8 0 45.3l-79.7 79.7c-12.5 12.5-32.8 12.5-45.3 0L153.7 131c-12.5-12.5-12.5-32.8 0-45.3l74.1-74.1zM131 153.7l79.7 79.7c12.5 12.5 12.5 32.8 0 45.3L131 358.3c-12.5 12.5-32.8 12.5-45.3 0L11.7 284.3c-15.6-15.6-15.6-40.9 0-56.6l74.1-74.1c12.5-12.5 32.8-12.5 45.3 0zM153.7 381l79.7-79.7c12.5-12.5 32.8-12.5 45.3 0L358.3 381c12.5 12.5 12.5 32.8 0 45.3l-74.1 74.1c-15.6 15.6-40.9 15.6-56.6 0l-74.1-74.1c-12.5-12.5-12.5-32.8 0-45.3zM381 358.3l-79.7-79.7c-12.5-12.5-12.5-32.8 0-45.3L381 153.7c12.5-12.5 32.8-12.5 45.3 0l74.1 74.1c15.6 15.6 15.6 40.9 0 56.6l-74.1 74.1c-12.5 12.5-32.8 12.5-45.3 0z";
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
    exports.faDiamonds4 = exports.definition;
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

// src/_entries/faDiamonds4.js
var import_faDiamonds4 = __toESM(require_faDiamonds4());
var export_faDiamonds4 = import_faDiamonds4.definition;
export {
  export_faDiamonds4 as faDiamonds4
};
