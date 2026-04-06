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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMelon.js
var require_faMelon = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMelon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "melon";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e310";
    var svgPathData = "M256 512c-38.5 0-75-8.5-107.7-23.7c67.1-5.8 149.8-48.2 220.8-119.2s113.4-153.8 119.2-220.8C503.5 181 512 217.5 512 256c0 141.4-114.6 256-256 256zM74.7 436.8c-19.1-19.4-26.8-55.8-11.9-108c14.6-51.2 49.6-110.2 102.7-163.3s112.1-88.1 163.3-102.7c52.4-15 88.9-7.1 108.2 12.2s27.2 55.9 12.2 108.2c-14.6 51.2-49.6 110.2-102.7 163.3s-112.1 88.1-163.3 102.7c-52.1 14.9-88.6 7.2-108-11.9c-.2-.2-.4-.4-.5-.5zm-51-73.1C8.5 331 0 294.5 0 256C0 114.6 114.6 0 256 0c38.5 0 75 8.5 107.7 23.7c-67.1 5.8-149.8 48.2-220.8 119.2s-113.4 153.8-119.2 220.8z";
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
    exports.faMelon = exports.definition;
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

// src/_entries/faMelon.js
var import_faMelon = __toESM(require_faMelon());
var export_faMelon = import_faMelon.definition;
export {
  export_faMelon as faMelon
};
