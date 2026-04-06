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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLocationArrowUp.js
var require_faLocationArrowUp = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLocationArrowUp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "location-arrow-up";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e63a";
    var svgPathData = "M35.8 431.9L190.9 54.2C196.4 40.8 209.5 32 224 32s27.6 8.8 33.1 22.2L412.2 431.9c9.4 23-7.4 48.1-32.3 48.1c-7.8 0-15.3-2.6-21.4-7.3L224 368 89.4 472.7C83.3 477.4 75.8 480 68 480c-24.8 0-41.7-25.2-32.3-48.1z";
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
    exports.faLocationArrowUp = exports.definition;
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

// src/_entries/faLocationArrowUp.js
var import_faLocationArrowUp = __toESM(require_faLocationArrowUp());
var export_faLocationArrowUp = import_faLocationArrowUp.definition;
export {
  export_faLocationArrowUp as faLocationArrowUp
};
