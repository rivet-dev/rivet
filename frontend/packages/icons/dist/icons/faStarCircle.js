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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleStar.js
var require_faCircleStar = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleStar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-star";
    var width = 512;
    var height = 512;
    var aliases = ["star-circle"];
    var unicode = "e123";
    var svgPathData = "M0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zM256 104c-6.1 0-11.7 3.5-14.3 8.9l-36.2 73.4-81 11.8c-6 .9-11 5.1-12.9 10.9s-.3 12.2 4 16.4l58.6 57.2-13.8 80.7c-1 6 1.4 12.1 6.4 15.6s11.5 4.1 16.8 1.2L256 342.1l72.5 38.1c5.4 2.8 11.9 2.4 16.9-1.2s7.4-9.6 6.4-15.6l-13.8-80.7 58.6-57.2c4.4-4.3 5.9-10.6 4-16.4s-6.9-10-12.9-10.9l-81.1-11.8-36.2-73.4c-2.7-5.5-8.3-8.9-14.3-8.9z";
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
    exports.faCircleStar = exports.definition;
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

// src/_entries/faStarCircle.js
var import_faCircleStar = __toESM(require_faCircleStar());
var export_faStarCircle = import_faCircleStar.definition;
export {
  export_faStarCircle as faStarCircle
};
