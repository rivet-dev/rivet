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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPotFood.js
var require_faPotFood = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPotFood.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pot-food";
    var width = 640;
    var height = 512;
    var aliases = [127858];
    var unicode = "e43f";
    var svgPathData = "M112 24c0 26.5 10.5 38.7 25.9 56.1l.4 .4C154.8 99 176 122.9 176 168c0 13.3-10.7 24-24 24s-24-10.7-24-24c0-26.5-10.5-38.7-25.9-56.1l-.4-.4C85.2 93 64 69.1 64 24C64 10.7 74.7 0 88 0s24 10.7 24 24zm112 0c0 26.5 10.5 38.7 25.9 56.1l.4 .4C266.8 99 288 122.9 288 168c0 13.3-10.7 24-24 24s-24-10.7-24-24c0-26.5-10.5-38.7-25.9-56.1l-.4-.4C197.2 93 176 69.1 176 24c0-13.3 10.7-24 24-24s24 10.7 24 24zm208 88c50.7 0 93.6 33.7 107.4 80l-214.7 0c13.8-46.3 56.6-80 107.4-80zM77.7 320L48 320c-17.7 0-32-14.3-32-32s14.3-32 32-32l17.9 0c-.2-17.5 14-32 31.7-32l444.8 0c17.7 0 31.9 14.5 31.7 32l17.9 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-29.7 0c-20.6 64-66.1 116.7-124.7 147.2l-1.2 9.7c-2.5 20-19.5 35-39.7 35l-153.4 0c-20.2 0-37.2-15-39.7-35l-1.2-9.7C143.8 436.7 98.3 384 77.7 320z";
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
    exports.faPotFood = exports.definition;
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

// src/_entries/faPotFood.js
var import_faPotFood = __toESM(require_faPotFood());
var export_faPotFood = import_faPotFood.definition;
export {
  export_faPotFood as faPotFood
};
