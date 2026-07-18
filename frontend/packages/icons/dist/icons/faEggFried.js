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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faEggFried.js
var require_faEggFried = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faEggFried.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "egg-fried";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f7fc";
    var svgPathData = "M284.3 39.3c30.4 31.6 89.1 65.9 131.5 76.7c27.5 7 52.8 23.2 71 47.8c42.1 56.8 30.2 137-26.6 179.1c-17.8 13.2-37.9 21.1-58.4 24c-37.2 5.2-84.5 32.7-103.3 65.2c-44.2 76.5-142 102.7-218.6 58.6S-22.7 348.5 21.4 272c24.8-42.9 38.7-105.9 43.1-155.2C67.1 87 80.1 58 103.3 35.7c51-49 132-47.4 181 3.6zM336 240a112 112 0 1 0 -224 0 112 112 0 1 0 224 0zm-160-8c0 8.8-7.2 16-16 16s-16-7.2-16-16c0-39.8 32.2-72 72-72c8.8 0 16 7.2 16 16s-7.2 16-16 16c-22.1 0-40 17.9-40 40z";
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
    exports.faEggFried = exports.definition;
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

// src/_entries/faEggFried.js
var import_faEggFried = __toESM(require_faEggFried());
var export_faEggFried = import_faEggFried.definition;
export {
  export_faEggFried as faEggFried
};
