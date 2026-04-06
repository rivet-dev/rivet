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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faEggplant.js
var require_faEggplant = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faEggplant.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "eggplant";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e16c";
    var svgPathData = "M505 7c9.4 9.4 9.4 24.6 0 33.9l-9 9L496 192c0 8.8-7.2 16-16 16c-26.5 0-48-21.5-48-48c0-5.8-4-10.7-9.4-12.6c-13.3-4.8-25.4-13-35.1-22.8s-18-21.9-22.8-35.1c-2-5.4-6.9-9.4-12.6-9.4c-26.5 0-48-21.5-48-48c0-8.8 7.2-16 16-16l142.1 0 9-9c9.4-9.4 24.6-9.4 33.9 0zM169.9 139.3c40.3-19.6 81-39.3 113.4-66.6c11.8 20 32.1 34.4 55.9 38.3c6.7 14 15.8 26.3 25.8 36.2s22.2 19.1 36.2 25.8c5 30.4 27.1 55.1 56.2 63.8C410.6 362.7 311.1 512.2 176.4 512C79.9 511.9 1.4 431.3 0 336C-1.6 222.5 83.4 181.3 169.9 139.3z";
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
    exports.faEggplant = exports.definition;
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

// src/_entries/faEggplant.js
var import_faEggplant = __toESM(require_faEggplant());
var export_faEggplant = import_faEggplant.definition;
export {
  export_faEggplant as faEggplant
};
