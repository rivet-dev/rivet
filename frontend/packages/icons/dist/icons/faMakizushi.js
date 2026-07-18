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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSushiRoll.js
var require_faSushiRoll = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSushiRoll.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sushi-roll";
    var width = 448;
    var height = 512;
    var aliases = ["maki-roll", "makizushi"];
    var unicode = "e48b";
    var svgPathData = "M448 368l0-224C448 82.1 347.7 32 224 32S0 82.1 0 144L0 368c0 61.9 100.3 112 224 112s224-50.1 224-112zM308.2 198.4c-24.5 6.1-53.3 9.6-84.2 9.6c-40.7 0-77.8-6.1-106-16.1c14.8-14.8 32.8-26.7 51.4-36.1c20.5-10.3 40.9-17 56.9-20.8c18.2 27.3 46.6 49.9 82 63.4zM205.1 80.4c.8 6 2.1 11.9 3.8 17.7c-17.3 4.7-37.4 11.8-57.5 21.9c-24.5 12.3-49.9 29.4-70.3 52.7C70.2 164.1 64 154.4 64 144c0-32.8 61.7-59.8 141.1-63.6zm40.7 .2c78 4.3 138.2 31 138.2 63.4c0 9.8-5.5 19.1-15.4 27.4c-69.5-5.5-113.5-48.4-122.8-90.8z";
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
    exports.faSushiRoll = exports.definition;
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

// src/_entries/faMakizushi.js
var import_faSushiRoll = __toESM(require_faSushiRoll());
var export_faMakizushi = import_faSushiRoll.definition;
export {
  export_faMakizushi as faMakizushi
};
