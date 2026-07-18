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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faShredder.js
var require_faShredder = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faShredder.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "shredder";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f68a";
    var svgPathData = "M64 64C64 28.7 92.7 0 128 0L354.7 0c17 0 33.3 6.7 45.3 18.7L429.3 48c12 12 18.7 28.3 18.7 45.3l0 98.7c35.3 0 64 28.7 64 64l0 96c0 17.7-14.3 32-32 32L32 384c-17.7 0-32-14.3-32-32l0-96c0-35.3 28.7-64 64-64L64 64zM384 93.3L354.7 64 128 64l0 128 256 0 0-98.7zM432 296a24 24 0 1 0 0-48 24 24 0 1 0 0 48zM40 416l48 0 0 72c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-72zm96 0l48 0 0 72c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-72zm96 0l48 0 0 72c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-72zm96 0l48 0 0 72c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-72zm96 0l48 0 0 72c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-72z";
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
    exports.faShredder = exports.definition;
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

// src/_entries/faShredder.js
var import_faShredder = __toESM(require_faShredder());
var export_faShredder = import_faShredder.definition;
export {
  export_faShredder as faShredder
};
