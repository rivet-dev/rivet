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

// src/node_modules/@fortawesome/free-solid-svg-icons/faNairaSign.js
var require_faNairaSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faNairaSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "naira-sign";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e1f6";
    var svgPathData = "M122.6 46.3c-7.8-11.7-22.4-17-35.9-12.9S64 49.9 64 64l0 192-32 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l32 0 0 144c0 17.7 14.3 32 32 32s32-14.3 32-32l0-144 89.5 0 107.8 161.8c7.8 11.7 22.4 17 35.9 12.9S384 462.1 384 448l0-144 32 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-32 0 0-192c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 192-57.5 0-139.8-209.7zM294.5 304l25.5 0 0 38.3-25.5-38.3zM185.5 256l-57.5 0 0-86.3 57.5 86.3z";
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
    exports.faNairaSign = exports.definition;
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

// src/_entries/faNairaSign.js
var import_faNairaSign = __toESM(require_faNairaSign());
var export_faNairaSign = import_faNairaSign.definition;
export {
  export_faNairaSign as faNairaSign
};
