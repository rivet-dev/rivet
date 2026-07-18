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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPawClaws.js
var require_faPawClaws = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPawClaws.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "paw-claws";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f702";
    var svgPathData = "M178.1 15.3C183 9.9 192 13.4 192 20.7l0 50.5c18.3 9.8 34.3 28.9 42.5 53.6c14.3 42.9-.3 86.2-32.6 96.8s-70.1-15.6-84.4-58.5c-4-11.9-5.7-23.9-5.5-35.2c0 0 0 0 0 0c0-25.7 9.6-50.6 26.9-69.6l39.2-43.1zm132 206.3c-32.3-10.6-46.9-53.9-32.6-96.8c8.2-24.7 24.2-43.8 42.5-53.6l0-50.5c0-7.3 9-10.8 13.9-5.4l39.2 43.1c17.3 19 26.9 43.9 26.9 69.6c0 0 0 0 0 0c.2 11.3-1.5 23.2-5.5 35.1c-14.3 42.9-52.1 69.1-84.4 58.5zm-209.7 8.9c18.9 32.4 14.3 70.1-10.2 84.1s-59.7-.9-78.5-33.3C3.7 267.8-.1 253.3 0 240c0 0 0 0 0 0l0-3.2c0-13.5 4.3-26.7 12.2-37.6l37.3-51.3c4.6-6.3 14.5-3 14.5 4.7l0 44.2c13.9 6.1 27.1 17.8 36.4 33.7zM69.2 433.2C121.6 291.9 214.7 256 256 256s134.4 35.9 186.8 177.2c3.6 9.7 5.2 20.1 5.2 30.5l0 1.6c0 25.8-20.9 46.7-46.7 46.7c-11.5 0-22.9-1.4-34-4.2l-88-22c-15.3-3.8-31.3-3.8-46.6 0l-88 22c-11.1 2.8-22.5 4.2-34 4.2C84.9 512 64 491.1 64 465.3l0-1.6c0-10.4 1.6-20.7 5.2-30.5zM421.8 314.7c-24.5-14-29.1-51.7-10.2-84.1c9.3-15.9 22.4-27.6 36.4-33.7l0-44.2c0-7.7 9.9-11 14.5-4.7l37.3 51.3c8 10.9 12.2 24.1 12.2 37.6l0 3.2s0 0 0 0c.1 13.3-3.7 27.8-11.6 41.4c-18.9 32.4-54 47.3-78.5 33.3z";
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
    exports.faPawClaws = exports.definition;
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

// src/_entries/faPawClaws.js
var import_faPawClaws = __toESM(require_faPawClaws());
var export_faPawClaws = import_faPawClaws.definition;
export {
  export_faPawClaws as faPawClaws
};
