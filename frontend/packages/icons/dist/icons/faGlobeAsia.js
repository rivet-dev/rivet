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

// src/node_modules/@fortawesome/free-solid-svg-icons/faEarthAsia.js
var require_faEarthAsia = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faEarthAsia.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "earth-asia";
    var width = 512;
    var height = 512;
    var aliases = [127759, "globe-asia"];
    var unicode = "f57e";
    var svgPathData = "M50 284.8c4.2 2.1 9 3.2 14 3.2l50.7 0c8.5 0 16.6 3.4 22.6 9.4l13.3 13.3c6 6 14.1 9.4 22.6 9.4l18.7 0c17.7 0 32-14.3 32-32l0-40c0-13.3 10.7-24 24-24s24-10.7 24-24l0-42.7c0-8.5 3.4-16.6 9.4-22.6l13.3-13.3c6-6 9.4-14.1 9.4-22.6L304 57c0-1.2-.1-2.3-.2-3.5-15.4-3.6-31.4-5.5-47.8-5.5-114.9 0-208 93.1-208 208 0 9.8 .7 19.4 2 28.8zm403.3 37.3c-3.2-1.4-6.7-2.1-10.5-2.1L432 320c-8.8 0-16-7.2-16-16s-7.2-16-16-16l-34.7 0c-8.5 0-16.6 3.4-22.6 9.4l-45.3 45.3c-6 6-9.4 14.1-9.4 22.6l0 18.7c0 17.7 14.3 32 32 32l18.7 0c8.5 0 16.6 3.4 22.6 9.4 2.2 2.2 4.7 4.1 7.3 5.5 39.3-25.4 69.5-63.6 84.6-108.8zM0 256a256 256 0 1 1 512 0 256 256 0 1 1 -512 0zM128 368c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16zM272 256c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16s16-7.2 16-16l0-32c0-8.8-7.2-16-16-16zm48-112l0 32c0 8.8 7.2 16 16 16s16-7.2 16-16l0-32c0-8.8-7.2-16-16-16s-16 7.2-16 16z";
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
    exports.faEarthAsia = exports.definition;
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

// src/_entries/faGlobeAsia.js
var import_faEarthAsia = __toESM(require_faEarthAsia());
var export_faGlobeAsia = import_faEarthAsia.definition;
export {
  export_faGlobeAsia as faGlobeAsia
};
