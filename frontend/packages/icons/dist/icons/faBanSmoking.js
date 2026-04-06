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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBanSmoking.js
var require_faBanSmoking = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBanSmoking.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ban-smoking";
    var width = 512;
    var height = 512;
    var aliases = [128685, "smoking-ban"];
    var unicode = "f54d";
    var svgPathData = "M99.5 144.8l79.2 79.2-50.7 0c-17.7 0-32 14.3-32 32l0 32c0 17.7 14.3 32 32 32l146.7 0 92.5 92.5c-31.4 22.4-69.8 35.5-111.2 35.5-106 0-192-86-192-192 0-41.5 13.1-79.9 35.5-111.2zM333.3 288l-32-32 82.7 0 0 32-50.7 0zm32 32l18.7 0c17.7 0 32-14.3 32-32l0-32c0-17.7-14.3-32-32-32L269.3 224 144.8 99.5c31.4-22.4 69.8-35.5 111.2-35.5 106 0 192 86 192 192 0 41.5-13.1 79.9-35.5 111.2L365.3 320zM256 512a256 256 0 1 0 0-512 256 256 0 1 0 0 512zM272 96c-8.8 0-16 7.2-16 16 0 26.5 21.5 48 48 48l32 0c8.8 0 16 7.2 16 16s7.2 16 16 16 16-7.2 16-16c0-26.5-21.5-48-48-48l-32 0c-8.8 0-16-7.2-16-16s-7.2-16-16-16z";
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
    exports.faBanSmoking = exports.definition;
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

// src/_entries/faBanSmoking.js
var import_faBanSmoking = __toESM(require_faBanSmoking());
var export_faBanSmoking = import_faBanSmoking.definition;
export {
  export_faBanSmoking as faBanSmoking
};
