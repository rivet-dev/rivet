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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faShirtTankTop.js
var require_faShirtTankTop = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faShirtTankTop.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "shirt-tank-top";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e3c9";
    var svgPathData = "M112 32C112 14.3 97.7 0 80 0S48 14.3 48 32l0 32c0 91-19.2 143.6-33.3 170.1C6.7 249.1 0 265.5 0 282.5L0 448c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-165.5c0-17-6.7-33.5-14.7-48.4C355.2 207.6 336 155 336 64l0-32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 48c0 44.2-35.8 80-80 80s-80-35.8-80-80l0-48z";
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
    exports.faShirtTankTop = exports.definition;
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

// src/_entries/faShirtTankTop.js
var import_faShirtTankTop = __toESM(require_faShirtTankTop());
var export_faShirtTankTop = import_faShirtTankTop.definition;
export {
  export_faShirtTankTop as faShirtTankTop
};
