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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPickaxe.js
var require_faPickaxe = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPickaxe.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pickaxe";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5bf";
    var svgPathData = "M462.4 373.5s0 0 0 0l-.1-.2-.3-.8c-.3-.7-.7-1.8-1.3-3.3c-1.2-2.9-3-7.2-5.5-12.7c-5-11-12.6-26.7-23.1-45.3c-21-37.3-53.6-86-99.5-132s-94.7-78.5-132-99.5c-18.6-10.5-34.3-18.1-45.3-23.1c-5.5-2.5-9.8-4.3-12.7-5.5c-1.4-.6-2.5-1-3.3-1.3l-.8-.3-.2-.1s0 0 0 0s0 0 0 0c-6.2-2.3-10.4-8.2-10.5-14.8s3.9-12.6 10-15.1C169.5 7 204 0 240 0c59.6 0 114.7 19.2 159.5 51.6l9.4-9.8c6-6.2 14.2-9.7 22.8-9.8s16.9 3.3 22.9 9.4l16 16c6.1 6.1 9.5 14.3 9.4 22.9s-3.6 16.8-9.8 22.8l-9.8 9.4C492.8 157.3 512 212.4 512 272c0 36-7 70.5-19.8 102c-2.5 6.1-8.5 10.1-15.1 10s-12.5-4.3-14.8-10.5c0 0 0 0 0 0zM9.4 502.6C-3 490.3-3.1 470.4 8.9 457.8l272-282.9c9.7 8.4 19.5 17.4 29.1 27s18.6 19.4 27 29.1L54.2 503.1c-12.6 12.1-32.5 11.9-44.8-.4z";
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
    exports.faPickaxe = exports.definition;
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

// src/_entries/faPickaxe.js
var import_faPickaxe = __toESM(require_faPickaxe());
var export_faPickaxe = import_faPickaxe.definition;
export {
  export_faPickaxe as faPickaxe
};
