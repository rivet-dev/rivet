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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFacePleading.js
var require_faFacePleading = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFacePleading.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-pleading";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e386";
    var svgPathData = "M0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zM302.7 427.9c6.6-5.9 7.1-16 1.2-22.6C294.7 395.1 278.7 384 256 384s-38.7 11.1-47.9 21.3c-5.9 6.6-5.4 16.7 1.2 22.6s16.7 5.4 22.6-1.2c4.7-5.3 12.7-10.7 24.1-10.7s19.4 5.4 24.1 10.7c5.9 6.6 16 7.1 22.6 1.2zM240 240c0-41.5-31.5-75.6-72-79.6c10-10.8 17.6-24.1 21.9-38.8l1.5-5.2c2.4-8.5-2.5-17.4-11-19.8s-17.4 2.5-19.8 11l-1.5 5.2c-7 24.7-28.1 42.7-53.6 45.9L94 160.1c-8.8 1.1-15 9.1-13.9 17.9c1 8.1 7.9 14 15.9 14c-10 13.4-16 30-16 48c0 44.2 35.8 80 80 80s80-35.8 80-80zm192 0c0-18-6-34.6-16-48c8 0 14.9-5.9 15.9-14c1.1-8.8-5.1-16.8-13.9-17.9l-11.5-1.4c-25.5-3.2-46.6-21.3-53.6-45.9l-1.5-5.2c-2.4-8.5-11.3-13.4-19.8-11s-13.4 11.3-11 19.8l1.5 5.2c4.2 14.8 11.8 28 21.9 38.8c-40.4 4-72 38.1-72 79.6c0 44.2 35.8 80 80 80s80-35.8 80-80zm-304-8c17.7 0 32-14.3 32-32c0-5.8-1.5-11.2-4.2-15.8c1.4-.1 2.8-.2 4.2-.2c30.9 0 56 25.1 56 56s-25.1 56-56 56s-56-25.1-56-56c0-5.7 .8-11.2 2.4-16.4c5.7 5.2 13.3 8.4 21.6 8.4zm224-32c0-5.8-1.5-11.2-4.2-15.8c1.4-.1 2.8-.2 4.2-.2c30.9 0 56 25.1 56 56s-25.1 56-56 56s-56-25.1-56-56c0-5.7 .8-11.2 2.4-16.4c5.7 5.2 13.3 8.4 21.6 8.4c17.7 0 32-14.3 32-32z";
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
    exports.faFacePleading = exports.definition;
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

// src/_entries/faFacePleading.js
var import_faFacePleading = __toESM(require_faFacePleading());
var export_faFacePleading = import_faFacePleading.definition;
export {
  export_faFacePleading as faFacePleading
};
