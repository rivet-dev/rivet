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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleEuro.js
var require_faCircleEuro = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleEuro.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-euro";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5ce";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM128 272l16 0c-.6-5-.9-10.1-.9-15.2c0-5.7 .4-11.3 1.1-16.8L128 240c-8.8 0-16-7.2-16-16s7.2-16 16-16l24.7 0c19.2-46.9 65.4-80 119.2-80l41.1 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-41.1 0c-26.3 0-49.6 12.6-64.4 32l80.5 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-95.1 0c-1.1 5.4-1.7 11-1.7 16.8c0 5.2 .5 10.3 1.4 15.2l95.4 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-81.6 0c14.7 20.3 38.5 33.5 65.5 33.5l41.1 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-41.1 0c-54.4 0-101-33.8-119.8-81.5L128 304c-8.8 0-16-7.2-16-16s7.2-16 16-16z";
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
    exports.faCircleEuro = exports.definition;
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

// src/_entries/faCircleEuro.js
var import_faCircleEuro = __toESM(require_faCircleEuro());
var export_faCircleEuro = import_faCircleEuro.definition;
export {
  export_faCircleEuro as faCircleEuro
};
