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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCampfire.js
var require_faCampfire = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCampfire.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "campfire";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f6ba";
    var svgPathData = "M279.1 43.9L262.1 25.8c-1.8-1.9-3.6-4-5.4-6.1c0 0 0 0 0 0C248.6 10.4 239.9 .3 227.2 0c-6.5-.2-13.1 2.1-18.3 6.8c-21.1 19.2-45 44.4-63.8 72.9c-18.7 28.3-33.1 61-33.1 95C112 255.2 175.4 320 256 320c79.8 0 144-64.7 144-145.3c0-27.3-9.9-55.4-23.6-80.1C362.7 69.7 344.7 47 326.9 30.4c-5-4.7-11.5-7.1-17.9-7.1c-6.8 0-14 2.5-18.8 8.1l-11 12.5zm36.5 174.4c0 32.1-26 58.2-58.2 58.2s-58.2-26-58.2-58.2c0-33.2 33.7-66.4 49.8-80.4c4.9-4.2 11.9-4.2 16.8 0c16.1 14 49.8 47.2 49.8 80.4zM42.8 289.9c-16.6-5.9-35 2.7-40.9 19.4s2.7 35 19.4 40.9L160.9 400 21.2 449.9C4.6 455.8-4.1 474.1 1.9 490.8s24.3 25.3 40.9 19.4L256 434l213.2 76.2c16.6 5.9 35-2.7 40.9-19.4s-2.7-35-19.4-40.9L351.1 400l139.6-49.9c16.6-5.9 25.3-24.3 19.4-40.9s-24.3-25.3-40.9-19.4L256 366 42.8 289.9z";
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
    exports.faCampfire = exports.definition;
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

// src/_entries/faCampfire.js
var import_faCampfire = __toESM(require_faCampfire());
var export_faCampfire = import_faCampfire.definition;
export {
  export_faCampfire as faCampfire
};
