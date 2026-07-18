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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPotato.js
var require_faPotato = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPotato.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "potato";
    var width = 512;
    var height = 512;
    var aliases = [129364];
    var unicode = "e440";
    var svgPathData = "M466.3 249.2C494.4 222.9 512 185.5 512 144C512 64.5 447.5 0 368 0c-39.8 0-75.9 16.2-102 42.3C238.6 69.8 205 95 170.9 113.6c-24.1 13.2-44 33.1-57.3 57.3C95 205 69.8 238.6 42.3 266C16.2 292.1 0 328.2 0 368c0 79.5 64.5 144 144 144c30.3 0 58.4-9.3 81.5-25.3c28.1-19.3 60.2-35.3 92.5-46.2c44.1-14.9 80.1-50.9 93.1-99.2c9.2-34.2 29.4-67.9 55.2-92.1zM384 168a24 24 0 1 1 0 48 24 24 0 1 1 0-48zM192 200a24 24 0 1 1 0 48 24 24 0 1 1 0-48zM136 416a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faPotato = exports.definition;
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

// src/_entries/faPotato.js
var import_faPotato = __toESM(require_faPotato());
var export_faPotato = import_faPotato.definition;
export {
  export_faPotato as faPotato
};
