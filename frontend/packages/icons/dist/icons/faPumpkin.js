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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPumpkin.js
var require_faPumpkin = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPumpkin.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pumpkin";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f707";
    var svgPathData = "M234.6 103.9C251.5 98.8 269.4 96 288 96c22.8 0 44.7 4.2 64.8 11.7c-.6-2-.8-4.1-.8-6.3l0-66.1c0-6.9-4.1-13.2-10.5-15.9L299.3 1.4c-2.2-.9-4.5-1.4-6.8-1.4l-1.2 0c-6.9 0-13.1 4.1-15.8 10.4l-40.9 93.5zm161.6 27.3c12.1 8.8 23.1 19.1 32.7 30.5c5.6 6.7-2.9 14.6-10.4 10.1C380.3 149 335.7 136 288 136s-92.3 13-130.5 35.7c-7.5 4.5-16-3.4-10.4-10.1c9.6-11.4 20.6-21.7 32.7-30.5c-.6-.1-1.2-.2-1.8-.3c-8.5-1.8-17.2-2.8-26-2.8C68.1 128 0 214 0 320S68.1 512 152 512c19.3 0 37.8-4.5 54.7-12.8c8.3-4.1 18.2-4.1 26.5 0c17 8.3 35.4 12.8 54.7 12.8s37.8-4.5 54.7-12.8c8.3-4.1 18.2-4.1 26.5 0c17 8.3 35.4 12.8 54.7 12.8c83.9 0 152-86 152-192s-68.1-192-152-192c-8.9 0-17.6 1-26 2.8c-.6 .1-1.2 .2-1.8 .3z";
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
    exports.faPumpkin = exports.definition;
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

// src/_entries/faPumpkin.js
var import_faPumpkin = __toESM(require_faPumpkin());
var export_faPumpkin = import_faPumpkin.definition;
export {
  export_faPumpkin as faPumpkin
};
