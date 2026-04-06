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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMosque.js
var require_faMosque = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMosque.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mosque";
    var width = 576;
    var height = 512;
    var aliases = [128332];
    var unicode = "f678";
    var svgPathData = "M174.8 224l226.4 0c43.5 0 78.8-35.3 78.8-78.8 0-25.5-12.3-49.4-33.1-64.2L297.3-25.4c-5.6-3.9-13-3.9-18.5 0L129.1 81C108.3 95.8 96 119.7 96 145.2 96 188.7 131.3 224 174.8 224zM512 512c35.3 0 64-28.7 64-64l0-224c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 48-448 0 0-48c0-17.7-14.3-32-32-32S0 206.3 0 224L0 448c0 35.3 28.7 64 64 64l448 0zM240 384c0-26.5 21.5-48 48-48s48 21.5 48 48l0 80-96 0 0-80z";
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
    exports.faMosque = exports.definition;
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

// src/_entries/faMosque.js
var import_faMosque = __toESM(require_faMosque());
var export_faMosque = import_faMosque.definition;
export {
  export_faMosque as faMosque
};
