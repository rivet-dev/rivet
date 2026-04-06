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

// src/node_modules/@fortawesome/free-solid-svg-icons/faRubleSign.js
var require_faRubleSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faRubleSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ruble-sign";
    var width = 448;
    var height = 512;
    var aliases = [8381, "rouble", "rub", "ruble"];
    var unicode = "f158";
    var svgPathData = "M112 32C94.3 32 80 46.3 80 64l0 208-40 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l40 0 0 48-40 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l40 0 0 32c0 17.7 14.3 32 32 32s32-14.3 32-32l0-32 152 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-152 0 0-48 112 0c79.5 0 144-64.5 144-144S335.5 32 256 32L112 32zM256 256l-112 0 0-160 112 0c44.2 0 80 35.8 80 80s-35.8 80-80 80z";
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
    exports.faRubleSign = exports.definition;
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

// src/_entries/faRub.js
var import_faRubleSign = __toESM(require_faRubleSign());
var export_faRub = import_faRubleSign.definition;
export {
  export_faRub as faRub
};
