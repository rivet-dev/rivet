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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLeafHeart.js
var require_faLeafHeart = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLeafHeart.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "leaf-heart";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f4cb";
    var svgPathData = "M272 96s0 0 0 0l104 0c42.7 0 79.9-23.9 98.8-59.1c3.5-6.5 13.6-6.8 16.2 .1c13.5 36.5 21.1 77 21.1 119.6C512 317.1 405.1 447.3 273 448l-1 0c-81.7 0-150.4-55.7-170.2-131.2C68.7 347.5 48 391.4 48 440l0 16c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-16c0-75.1 38.3-141.2 96.4-179.9C102.5 168.5 178.8 96 272 96c0 0 0 0 0 0zM189.7 242.8c0 14.4 5.8 28.3 16.2 38.4l70.9 69c6.2 6 16.1 6 22.3 0l70.9-69c10.4-10.1 16.2-23.9 16.2-38.4c0-29.5-23.9-53.5-53.5-53.5l-1.5 0c-14.3 0-28.1 5.6-38.4 15.6l-4.9 4.8-4.9-4.8c-10.3-10-24-15.6-38.4-15.6l-1.5 0c-29.5 0-53.5 23.9-53.5 53.5z";
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
    exports.faLeafHeart = exports.definition;
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

// src/_entries/faLeafHeart.js
var import_faLeafHeart = __toESM(require_faLeafHeart());
var export_faLeafHeart = import_faLeafHeart.definition;
export {
  export_faLeafHeart as faLeafHeart
};
