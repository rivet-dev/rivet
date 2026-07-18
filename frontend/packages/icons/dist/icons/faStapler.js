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

// src/node_modules/@fortawesome/free-solid-svg-icons/faStapler.js
var require_faStapler = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faStapler.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "stapler";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5af";
    var svgPathData = "M640 299.3L640 432c0 26.5-21.5 48-48 48L64 480c-17.7 0-32-14.3-32-32s14.3-32 32-32l384 0 0-48-352 0c-17.7 0-32-14.3-32-32l0-116.6-30.2-5.4c-19.6-3.5-33.8-20.5-33.8-40.4 0-8.9 2.9-17.5 8.2-24.6l35.6-47.5c32.8-43.8 84.4-69.6 139.1-69.6 27 0 53.6 6.3 77.8 18.4L586.9 213.5C619.5 229.7 640 263 640 299.3zM448 304l0-16-320-57.1 0 73.1 320 0z";
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
    exports.faStapler = exports.definition;
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

// src/_entries/faStapler.js
var import_faStapler = __toESM(require_faStapler());
var export_faStapler = import_faStapler.definition;
export {
  export_faStapler as faStapler
};
