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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSaluting.js
var require_faFaceSaluting = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSaluting.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-saluting";
    var width = 640;
    var height = 512;
    var aliases = [129761];
    var unicode = "e484";
    var svgPathData = "M258.1 58.5s0 0 0 0c14.5-9.8 18.4-29.4 8.8-44c0 0 0 0 0 0c-.1-.1-.2-.3-.3-.4C256.7-.6 236.8-4.4 222.2 5.5L35.1 132.5c-44.3 30.1-47.1 94.4-5.6 128.2l1.5 1.2c18.9 15.4 43.2 20.9 66 16.5c4.4-.9 8.8-2.1 13.1-3.8c.3-.1 .7-.3 1-.4c.5-.2 1-.4 1.5-.6c5.6-2.3 10.9-5.4 16-9l82.3-59.9s0 0 0 0c14.3-10.4 17.5-30.4 7.1-44.7c-4.8-6.7-11.8-10.9-19.3-12.5c-3.5-.7-7.2-.9-10.8-.4c-5.1 .7-10.1 2.6-14.6 5.8L128 185.8l0-39.1 27.3-18.5 13.1-8.9 89.7-60.9zM352 0c-18.4 0-36.3 1.9-53.5 5.6c12.6 27.9 3.8 61.6-22.4 79.4l-54.6 37c8.6 4.4 16.3 10.9 22.3 19.2c8.6 11.9 12.6 25.7 12.2 39.3c4.8-2.8 10.4-4.5 16.3-4.5c17.7 0 32 14.3 32 32s-14.3 32-32 32c-13.9 0-25.8-8.9-30.2-21.4c-3.5 4.4-7.7 8.5-12.5 12l-82.3 59.9c-13.9 10.1-29.5 16.6-45.6 19.6C126.5 425.5 229.1 512 352 512c141.4 0 256-114.6 256-256S493.4 0 352 0zM257.4 341c17.5 12.6 48.9 27 94.6 27s77.2-14.4 94.6-27c7.2-5.2 17.2-3.5 22.3 3.6s3.5 17.2-3.6 22.3c-22.7 16.4-60.7 33-113.4 33s-90.6-16.6-113.4-33c-7.2-5.2-8.8-15.2-3.6-22.3s15.2-8.8 22.3-3.6zm175-165a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faFaceSaluting = exports.definition;
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

// src/_entries/faFaceSaluting.js
var import_faFaceSaluting = __toESM(require_faFaceSaluting());
var export_faFaceSaluting = import_faFaceSaluting.definition;
export {
  export_faFaceSaluting as faFaceSaluting
};
