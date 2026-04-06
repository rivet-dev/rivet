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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPepper.js
var require_faPepper = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPepper.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pepper";
    var width = 512;
    var height = 512;
    var aliases = [129745];
    var unicode = "e432";
    var svgPathData = "M252.6 11.7C245.8 .3 231-3.4 219.7 3.4s-15.1 21.6-8.2 32.9l10.3 17.2c6.7 11.2 10.3 24 10.3 37l0 10.9c-5-1.8-10.1-3.1-15.3-4l-.9-.2c-8.8-1.5-17.9-1.7-27.1-.6c-4 .5-7.9 1.2-11.9 2.2c-51.4 12.9-74.7 65-61.9 116.4l37.1 148.3c2.1 8.6-3.1 17.3-11.6 19.4s-17.3-3.1-19.4-11.6L83.8 223c-11.8-47 .4-94.4 31.3-125.2c-13.6-2.8-28-2.6-42.4 1C21.3 111.7-10 163.9 2.9 215.3l56 224c12.9 51.4 65 82.7 116.4 69.8c7-1.7 13.6-4.2 19.8-7.3c5.6-2.8 12.3-2.8 17.9 0c3 1.5 6 2.8 9.2 4c10.7 4 22.2 6.2 34 6.2c11.6 0 22.9-2.1 33.5-6.1c3.2-1.2 6.4-2.6 9.4-4.1c5.6-2.8 12.3-2.8 17.9 0c6.2 3.1 12.8 5.6 19.8 7.3c51.4 12.9 103.6-18.4 116.4-69.8l56-224c12.9-51.4-18.4-103.6-69.8-116.4c-23.2-5.8-46.5-2.6-66.3 7.3c-5.6 2.8-12.3 2.8-17.9 0c-6.2-3.1-12.8-5.6-19.8-7.3c-19-4.8-38.1-3.5-55.3 2.6l0-10.9c0-21.7-5.9-43.1-17.1-61.7L252.6 11.7z";
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
    exports.faPepper = exports.definition;
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

// src/_entries/faPepper.js
var import_faPepper = __toESM(require_faPepper());
var export_faPepper = import_faPepper.definition;
export {
  export_faPepper as faPepper
};
