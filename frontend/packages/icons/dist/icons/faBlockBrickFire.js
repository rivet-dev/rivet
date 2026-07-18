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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBlockBrickFire.js
var require_faBlockBrickFire = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBlockBrickFire.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "block-brick-fire";
    var width = 640;
    var height = 512;
    var aliases = ["firewall"];
    var unicode = "e3dc";
    var svgPathData = "M128 32l256 0 0 80-256 0 0-80zM96 112l-64 0 0-16c0-35.3 28.7-64 64-64l0 80zM32 144l208 0 0 96L32 240l0-96zm0 224l0-96 64 0 0 96-64 0zm0 32l208 0 0 80L96 480c-35.3 0-64-28.7-64-64l0-16zm240 0l22.5 0c8 30 22.9 57.4 43.1 80L272 480l0-80zM480 144l0 .6-.6-.6 .6 0zm-208 0l135.9 0c-32.5 29.8-60.4 63.2-81 96L272 240l0-96zM128 272l181 0c-12.5 25.7-21 52.7-21 78.1c0 6 .3 12 .8 17.9L128 368l0-96zM480 112l-64 0 0-80c35.3 0 64 28.7 64 64l0 16zm25.7 96.3L518 194.5c5.4-6.1 13.3-8.8 20.9-8.9c7.2 0 14.3 2.6 19.9 7.8c19.7 18.3 39.8 43.2 55 70.6C629 291.2 640 322.2 640 352.2C640 440.8 568.7 512 480 512c-89.6 0-160-71.3-160-159.8c0-37.3 16-73.4 36.8-104.5c20.9-31.3 47.5-59 70.9-80.2c5.7-5.2 13.1-7.7 20.3-7.5c14.1 .3 23.8 11.4 32.7 21.6c0 0 0 0 0 0c2 2.3 4 4.6 6 6.7l19 19.9zM544 400.2c0-36.5-37-73-54.8-88.4c-5.4-4.7-13.1-4.7-18.5 0C453 327.1 416 363.6 416 400.2c0 35.3 28.7 64 64 64s64-28.7 64-64z";
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
    exports.faBlockBrickFire = exports.definition;
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

// src/_entries/faBlockBrickFire.js
var import_faBlockBrickFire = __toESM(require_faBlockBrickFire());
var export_faBlockBrickFire = import_faBlockBrickFire.definition;
export {
  export_faBlockBrickFire as faBlockBrickFire
};
