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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAppleCore.js
var require_faAppleCore = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAppleCore.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "apple-core";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e08f";
    var svgPathData = "M192 112c-8.8 0-16-7.2-16-16l0-16c0-44.2 35.8-80 80-80l16 0c8.8 0 16 7.2 16 16l0 16c0 44.2-35.8 80-80 80l-16 0zM30.5 167c-8-6.5-9.3-18.7-.7-24.3C43.9 133.4 60.7 128 80 128c27.3 0 59.7 10.3 82.7 19.3c18.8 7.3 39.9 7.3 58.7 0c22.9-8.9 55.4-19.3 82.7-19.3c19.3 0 36.1 5.4 50.3 14.7c8.6 5.6 7.3 17.9-.7 24.3c-40 32.3-65.5 81.7-65.5 137c0 54.4 24.7 103 63.4 135.3c7 5.9 8.8 16.3 2.7 23.1C325.8 493.7 290.9 512 256 512c-16.5 0-38.1-6.6-51.5-11.3c-8.1-2.8-16.9-2.8-25 0c-13.4 4.7-35 11.3-51.5 11.3c-34.9 0-69.8-18.3-98.1-49.6c-6.2-6.8-4.4-17.3 2.7-23.1C71.3 407 96 358.4 96 304c0-55.4-25.6-104.7-65.5-137z";
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
    exports.faAppleCore = exports.definition;
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

// src/_entries/faAppleCore.js
var import_faAppleCore = __toESM(require_faAppleCore());
var export_faAppleCore = import_faAppleCore.definition;
export {
  export_faAppleCore as faAppleCore
};
