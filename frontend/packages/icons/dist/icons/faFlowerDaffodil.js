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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFlowerDaffodil.js
var require_faFlowerDaffodil = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFlowerDaffodil.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "flower-daffodil";
    var width = 512;
    var height = 512;
    var aliases = [9880];
    var unicode = "f800";
    var svgPathData = "M288 281.3c9.8 4.3 20.6 6.7 32 6.7c44.2 0 80-35.8 80-80c0-26.2-12.6-49.4-32-64c19.4-14.6 32-37.8 32-64c0-44.2-35.8-80-80-80c-26.2 0-49.4 12.6-64 32C241.4 12.6 218.2 0 192 0c-44.2 0-80 35.8-80 80c0 26.2 12.6 49.4 32 64c-19.4 14.6-32 37.8-32 64c0 44.2 35.8 80 80 80c11.4 0 22.2-2.4 32-6.7l0 113.4C192.1 349.5 139.5 320 80 320l-50.5 0C13.2 320 0 333.2 0 349.5C0 439.3 72.7 512 162.5 512l61.5 0 32 0 32 0 61.5 0C439.3 512 512 439.3 512 349.5c0-16.3-13.2-29.5-29.5-29.5L432 320c-59.5 0-112.1 29.5-144 74.8l0-113.4zM208 144a48 48 0 1 1 96 0 48 48 0 1 1 -96 0z";
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
    exports.faFlowerDaffodil = exports.definition;
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

// src/_entries/faFlowerDaffodil.js
var import_faFlowerDaffodil = __toESM(require_faFlowerDaffodil());
var export_faFlowerDaffodil = import_faFlowerDaffodil.definition;
export {
  export_faFlowerDaffodil as faFlowerDaffodil
};
